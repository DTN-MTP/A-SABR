extern crate alloc;
use core::mem::{MaybeUninit, take};

use alloc::vec::Vec;
use itertools::Either;
use replace_with::replace_with_or_default_and_return as replace_with;

pub use crate::contact_manager::lex::StandardManagersDyn as CMDynStandard;
use crate::types::AnyNumber;

/// Types for which it is usefull to implement Parse<T> TryInto<T> in order to parse a full contact plan.
pub mod parsables {
    // pub use crate::{
    // contact::ContactInfo,
    // contact_manager::{
    //     legacy::lex::{Budget as CMLegacyBudget, Info as CMLegacyInfo, Kind as CMLegacyKind},
    //     segmentation::lex::{Info as CMSegmentInfo, Kind as CMSegmenKind},
    // },
    // node::NodeInfo,
    // types::AnyNumber,
    // vnode::VirtualNodeInfo,
    // };
}

#[derive(Clone, Copy, Debug)]
pub struct Located<T> {
    pub data: T,
    pub(crate) line: usize,
    pub(crate) toknum: usize,
}

#[derive(Clone, Copy, Debug)]
pub enum Delimiter {
    Open,
    Close,
    Separator,
}

impl TryFrom<&str> for Delimiter {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "[" => Ok(Delimiter::Open),
            "]" => Ok(Delimiter::Close),
            "," => Ok(Delimiter::Separator),
            _ => Err(()),
        }
    }
}

impl<T> Located<T> {
    pub fn err(self, e: &'static str) -> Located<&'static str> {
        Located {
            data: e,
            line: self.line,
            toknum: self.toknum,
        }
    }
}

pub trait Parse: Sized {
    type Token: Clone;
    type Parser: Default;

    const NOFEED: bool = false;

    /// Finalise a Parser to get self or an error
    fn parse(p: Self::Parser) -> Result<Self, &'static str>;

    /// Get a token and update the parser accordingly. return Ok(false) if more token needed, or Ok(true) to parse.
    fn feed(tok: Self::Token, parser: &mut Self::Parser) -> Result<bool, &'static str>;
}

pub trait LexFrom<T: ?Sized>: Parse {
    fn lex(t: &T, p: &Self::Parser) -> Result<Self::Token, &'static str>;
}

pub const ETYPE: &str = "Wrong type for next tocken.";
pub const EOF: &str = "Unexpected end of input while declaration was unfinished";
pub const MORON: &str = "This parser is in a improper state or was feed an improper token for the attempted operation. Please report a bug or stop playing with them directly";

#[derive(Clone, Copy, Debug)]
pub enum Partial<T1: Parse, T2: Parse> {
    None(T1::Parser),
    Fst(T1, T2::Parser),
}

#[derive(Clone, Copy, Debug)]
pub enum Delimited<T> {
    Delim(Delimiter),
    Val(T),
}

impl<T1: Parse, T2: Parse> Default for Partial<T1, T2> {
    fn default() -> Self {
        Self::None(Default::default())
    }
}

impl<T1: Parse, T2: Parse> Parse for (T1, T2) {
    type Token = Either<T1::Token, T2::Token>;
    type Parser = Partial<T1, T2>;

    fn parse(p: Self::Parser) -> Result<Self, &'static str> {
        match p {
            Partial::None(p) => Ok((T1::parse(p)?, T2::parse(Default::default())?)),
            Partial::Fst(t1, p) => Ok((t1, T2::parse(p)?)),
        }
    }

    fn feed(
        tok: Either<<T1 as Parse>::Token, <T2 as Parse>::Token>,
        parser: &mut Partial<T1, T2>,
    ) -> Result<bool, &'static str> {
        replace_with(parser, |parser| match (parser, tok) {
            (Partial::None(mut sub), Either::Left(tok)) => match T1::feed(tok, &mut sub) {
                Err(e) => (Err(e), Partial::None(sub)),
                Ok(false) => (Ok(false), Partial::None(sub)),
                Ok(true) => match T1::parse(sub) {
                    Err(e) => (Err(e), Default::default()),
                    Ok(v) => (Ok(T2::NOFEED), Partial::Fst(v, Default::default())),
                },
            },
            (Partial::Fst(fst, mut sub), Either::Right(tok)) => {
                (T2::feed(tok, &mut sub), Partial::Fst(fst, sub))
            }
            (parser, _) => (Err(MORON), parser),
        })
    }
    const NOFEED: bool = T1::NOFEED && T2::NOFEED;
}

impl<T1: Parse, T2: Parse, Src: ?Sized> LexFrom<Src> for (T1, T2)
where
    T1: LexFrom<Src>,
    T2: LexFrom<Src>,
{
    fn lex(t: &Src, p: &Self::Parser) -> Result<Self::Token, &'static str> {
        match p {
            Partial::None(parser) => Ok(Either::Left(T1::lex(t, parser)?)),
            Partial::Fst(_, parser) => Ok(Either::Right(T2::lex(t, parser)?)),
        }
    }
}

#[derive(Debug)]
pub struct VecBuilder<T: Parse> {
    parser: Option<T::Parser>,
    delim: bool,
    vec: Vec<T>,
}

impl<T: Parse> Default for VecBuilder<T> {
    fn default() -> Self {
        VecBuilder {
            parser: None,
            delim: false,
            vec: Vec::new(),
        }
    }
}

impl<T: Parse> Parse for Vec<T> {
    type Token = Delimited<T::Token>;

    type Parser = VecBuilder<T>;

    fn parse(p: VecBuilder<T>) -> Result<Self, &'static str> {
        Ok(p.vec)
    }

    fn feed(tok: Self::Token, parser: &mut Self::Parser) -> Result<bool, &'static str> {
        match (tok, parser) {
            (Delimited::Delim(Delimiter::Open), VecBuilder { parser, .. }) => {
                *parser = Some(Default::default());
                Ok(false)
            }
            (Delimited::Delim(Delimiter::Close), _) => Ok(true),
            (Delimited::Delim(Delimiter::Separator), VecBuilder { parser, delim, .. }) => {
                if *delim {
                    Err(MORON)
                } else {
                    *delim = true;
                    *parser = Some(Default::default());
                    Ok(false)
                }
            }
            (Delimited::Val(tok), VecBuilder { parser, vec, delim }) => {
                *delim = false;
                match parser {
                    None => {
                        return Err(MORON);
                    }
                    Some(par) => {
                        if T::feed(tok, par)? {
                            //Unwrap is ok because we just matched it with some
                            vec.push(T::parse(parser.take().unwrap())?);
                        }
                    }
                }
                Ok(false)
            }
        }
    }
}

impl<T: Parse, D: ?Sized> LexFrom<D> for Vec<T>
where
    T: LexFrom<D>,
    Delimiter: LexFrom<D>,
{
    fn lex(t: &D, p: &Self::Parser) -> Result<Self::Token, &'static str> {
        match (&p.parser, p.delim) {
            (None, false) => Ok(Delimited::Delim(Delimiter::lex(t, &None)?)),
            (None, true) => Err(MORON),
            (Some(p), true) => match Delimiter::lex(t, &None) {
                Ok(Delimiter::Close) => Ok(Delimited::Delim(Delimiter::Close)),
                _ => Ok(Delimited::Val(T::lex(t, p)?)),
            },
            (Some(p), false) => Ok(Delimited::Val(T::lex(t, p)?)),
        }
    }
}

#[macro_export]
macro_rules! empty_parse {
    ($T: ty) => {
        impl $crate::parsing::Parse for $T {
            type Token = ();
            type Parser = ();
            fn parse(_p: ()) -> Result<Self, &'static str> {
                Ok(core::default::Default::default())
            }
            fn feed(_tok: (), _parser: &mut ()) -> Result<bool, &'static str> {
                Err($crate::parsing::MORON)
            }
            const NOFEED: bool = true;
        }
        impl<T: ?Sized> $crate::parsing::LexFrom<T> for $T {
            fn lex(_t: &T, _p: &()) -> Result<(), &'static str> {
                Err("")
            }
        }
    };
}

empty_parse!(());

#[macro_export]
macro_rules! parse_single_tok {
    ($T: ty,$Tok: ty) => {
        impl $crate::parsing::Parse for $T {
            type Token = $Tok;
            type Parser = core::option::Option<$Tok>;
            fn parse(p: core::option::Option<$Tok>) -> core::result::Result<Self,&'static str> {
                match p.map(|tok| tok.try_into()) {
                    None => Err($crate::parsing::MORON),
                    Some(Err(_)) => Err(stringify!(Could not parse this $Tok into a $T)),
                    Some(Ok(val)) => Ok(val)
                }
            }
            fn feed(tok: $Tok, p: &mut core::option::Option<$Tok>) -> core::result::Result<bool,&'static str>{
                *p = Some(tok);
                Ok(true)
            }
        }

        impl<T: ?Sized> $crate::parsing::LexFrom<T> for $T
        where for<'a> &'a T: TryInto<$Tok>
        {
            fn lex(tok: &T, _p: &core::option::Option<$Tok>) -> core::result::Result<$Tok,&'static str>{
                match tok.try_into() {
                    Ok(tok) => Ok(tok),
                    Err(_) => Err(stringify!(Could not lex this into a $Tok))
                }
            }
        }
    };
}

parse_single_tok!(u16, AnyNumber);
parse_single_tok!(i8, AnyNumber);
parse_single_tok!(f64, AnyNumber);
parse_single_tok!(Delimiter, Delimiter);

#[macro_export]
macro_rules! choices {
    ($modname:ident,$name:ident,$(($names:ident,$types:ty)),*) => {
     pub mod $modname {
            use super::*;
            use $crate::{parse_single_tok, parsing::{LexFrom, MORON, Parse}};

            #[derive(Debug,Clone)]
            #[doc = stringify!(The diffenrent kinds of $name. Implement TryFrom<T> to parse them)]
            pub enum Kinds {
                $($names,)*
            }

            parse_single_tok!(Kinds,Kinds);

            #[derive(Debug,Clone)]
            #[doc(hidden)]
            pub enum Token {
                Keyword(Kinds),
                $($names(<$types as Parse>::Token),)*
            }
            #[doc(hidden)]
            #[derive(Debug,Default)]
            pub enum Parser {
                #[default]
                None,
                $($names(<$types as Parse>::Parser),)*
            }
            /// A choice of diffenrent things
            pub enum $name {
                $($names($types),)*
            }
            impl Parse for $name {
                type Token = Token;
                type Parser = Parser;
                fn parse(p: Self::Parser) -> core::result::Result<Self, &'static str> {
                    core::result::Result::Ok(match p {
                        Parser::None => return Err("No data given!"),
                        $(Parser::$names(p) => $name::$names(<$types as Parse>::parse(p)?),)*
                    })
                }
                fn feed(tok: Self::Token,mut parser: &mut Self::Parser) -> core::prelude::v1::Result<bool, &'static str> {
                    match (&mut parser,tok) {
                        (Parser::None,Token::Keyword(kind)) => {
                            match kind {
                                $(Kinds::$names => {
                                    *parser = Parser::$names(Default::default());
                                    Ok(<$types as Parse>::NOFEED)
                                })*
                            }
                        }
                        $(
                            (Parser::$names(parser),Token::$names(tok)) => <$types as Parse>::feed(tok,parser),
                        )*
                        _ => Err(MORON),
                    }
                }
            }
            impl<T: ?Sized> LexFrom<T> for $name
            where
                Kinds: LexFrom<T>,
                $($types: LexFrom<T>,)*
            {
                fn lex(t: &T, p: &Self::Parser) -> core::result::Result<Self::Token, &'static str> {
                    Ok(match p {
                        Parser::None => Token::Keyword(Kinds::lex(t, &None)?),
                        $(Parser::$names(p) => Token::$names(<$types as LexFrom<T>>::lex(t,p)?),)*
                    })
                }
            }
        }

    };
}

#[macro_export]
/// Use to implement Parse for a type whenever you want to parse as something else the try to convert using TryInto
/// parse_transparent(Dest,From) implement Parse for Dest, where From: Parse and Frop: TryInto<Dest>
macro_rules! parse_transparent {
    ($TD:ty,$TF:ty$(,$($Template:tt)*)?)  => {

        impl$(<$($Template)* >)? $crate::parsing::Parse for $TD {
            type Token = <$TF as $crate::parsing::Parse>::Token;

            type Parser = <$TF as $crate::parsing::Parse>::Parser;

            fn parse(p: Self::Parser) -> core::result::Result<Self, &'static str> {
                <$TF as $crate::parsing::Parse>::parse(p)?
                    .try_into()
                    .map_err(|_| stringify!(Could not convert the $TF into a $TD))
            }

            fn feed(tok: Self::Token, parser: &mut Self::Parser) -> core::result::Result<bool, &'static str> {
                <$TF as $crate::parsing::Parse>::feed(tok, parser)
            }
        }

        impl<$($($Template)* ,)? T: ?Sized> $crate::parsing::LexFrom<T> for $TD
        where
            $TF: $crate::parsing::LexFrom<T>,
        {
            fn lex(t: &T, p: &Self::Parser) -> core::result::Result<Self::Token, &'static str> {
                <$TF as $crate::parsing::LexFrom<T>>::lex(t, p)
            }
        }
    };
}

#[derive(Debug)]
pub struct ArrayParser<T: Parse, const N: usize>(T::Parser, [MaybeUninit<T>; N], pub usize);

impl<T: Parse, const N: usize> Default for ArrayParser<T, N> {
    fn default() -> Self {
        Self(Default::default(), MaybeUninit::uninit().into(), 0)
    }
}

impl<T: Parse, const N: usize> ArrayParser<T, N> {
    fn clear(&mut self, i: usize) {
        for j in 0..i {
            unsafe {
                self.1[j].assume_init_drop();
            }
        }
    }
}

impl<T: Parse, const N: usize> Parse for [T; N] {
    type Token = T::Token;
    type Parser = ArrayParser<T, N>;
    const NOFEED: bool = T::NOFEED;
    fn parse(mut p: Self::Parser) -> Result<Self, &'static str> {
        if Self::NOFEED {
            for i in 0..N {
                match T::parse(T::Parser::default()) {
                    Err(e) => {
                        p.clear(i);
                        return Err(e);
                    }
                    Ok(t) => p.1[i].write(t),
                };
            }
            let arr: MaybeUninit<_> = p.1.into();
            Ok(unsafe { arr.assume_init() })
        } else {
            if p.2 != N {
                p.clear(p.2);
                return Err("Array terminated while elements remain to be parsed");
            }
            Ok(unsafe { <_ as Into<MaybeUninit<_>>>::into(p.1).assume_init() })
        }
    }
    fn feed(tok: Self::Token, parser: &mut Self::Parser) -> Result<bool, &'static str> {
        match T::feed(tok, &mut parser.0) {
            Err(e) => {
                parser.clear(parser.2);
                Err(e)
            }
            Ok(true) => {
                let p = take(&mut parser.0);
                match T::parse(p) {
                    Err(e) => {
                        parser.clear(parser.2);
                        Err(e)
                    }
                    Ok(next) => {
                        parser.1[parser.2].write(next);
                        parser.2 += 1;
                        Ok(parser.2 == N)
                    }
                }
            }
            Ok(false) => Ok(false),
        }
    }
}

impl<S: ?Sized, T: LexFrom<S>, const N: usize> LexFrom<S> for [T; N] {
    fn lex(t: &S, p: &Self::Parser) -> Result<Self::Token, &'static str> {
        T::lex(t, &p.0)
    }
}

// well, two element tupple are ok, but what about 3? 4? more ? here it is (all based on the two elements implementation)
macro_rules! tupples_joy {
    ($Tf:ident,$($T:ident),*) => {
        impl<$($T:Parse, )* $Tf:Parse> Parse for ($($T,)* $Tf) {
            type Token = <( ($( $T, )*), $Tf) as Parse>::Token;
            type Parser = <( ($( $T, )*), $Tf) as Parse>::Parser;
            const NOFEED: bool = < ( ($( $T, )*) , $Tf) as Parse>::NOFEED;
            fn parse(p: Self::Parser) -> Result<Self, &'static str> {
                <( ($( $T, )*), $Tf) as Parse>::parse(p).map(
                    #[allow(non_snake_case)]
                    |(  ($($T,)*), $Tf )| ($($T,)* $Tf)
                )
            }
            fn feed(tok: Self::Token, parser: &mut Self::Parser) -> Result<bool, &'static str> {
                <( ($( $T, )*), $Tf) as Parse>::feed(tok,parser)

            }
        }
        impl<$($T:Parse, )* $Tf:Parse,T:?Sized> LexFrom<T> for ($($T,)* $Tf)
        where
            $($T: LexFrom<T>,)*
            $Tf: LexFrom<T>
        {
            fn lex(t: &T,p: &Self::Parser) -> Result<Self::Token,&'static str> {
                <( ($( $T, )*), $Tf) as LexFrom<T>>::lex(t,p)

            }
        }
    };
}

tupples_joy!(T1, T2, T3);
tupples_joy!(T1, T2, T3, T4);
tupples_joy!(T1, T2, T3, T4, T5);
tupples_joy!(T1, T2, T3, T4, T5, T6);
tupples_joy!(T1, T2, T3, T4, T5, T6, T7);
tupples_joy!(T1, T2, T3, T4, T5, T6, T7, T8);
tupples_joy!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
tupples_joy!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
tupples_joy!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
tupples_joy!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
tupples_joy!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
tupples_joy!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
tupples_joy!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15
);
tupples_joy!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);
tupples_joy!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17
);
tupples_joy!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18
);
tupples_joy!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19
);
tupples_joy!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20
);
