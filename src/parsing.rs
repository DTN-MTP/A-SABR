extern crate alloc;
use core::mem::{self, MaybeUninit, take};

use alloc::vec::Vec;
use itertools::Either;
use replace_with::replace_with_or_default_and_return as replace_with;

pub use crate::contact_manager::lex::StandardManagersDyn as CMDynStandard;
use crate::types::AnyNumber;

/// re-export of types for which it is usefull to implement Parse<T> TryInto<T> in order to parse a full contact plan.
pub mod parsables {
    pub use super::Delimiter;
    pub use crate::contact_manager::lex::StandardManagersKinds;
    pub use crate::contact_plan::from_asabr_lexer::ASABRPlanInfoKind;
    pub use crate::types::{AnyNumber, NodeID, NodeName};
}

// ***
// # Traits
// ***

/// The main parsing trait, building data from tokens.
/// Usually, you want to implement this trait using one of the macros in this crate, and not directly.
/// If you need to anyway, look at the doc of each of the members.
/// See also the LexFrom trait.
/// # Macro to implement the trait:
/// - `empty_parse` to make something not consume any token
/// - `choices` to recognise a identifier followed by matching data (basically an enum)
/// - `parse_single_tok` for data on a single token
/// - `parse_transparent` to parse something directly from something else (usefull using the types bellow to compose)
/// # Types wich already auto-implement the trait when the components does:
/// - Vec<T> when T: Parse  (Lexing require T and Delimiter)
/// - [T;N] when T: Parse
/// - Tupples, up to 20 elements each Parse-able
pub trait Parse: Sized {
    /// The kind of tokens needeed to build Self. Usually, a fitting enum.
    /// Implementing LexFrom will require to produce these
    type Token: Clone;
    /// The thing storing data during parsing.
    /// It need to implement Default
    type Parser: Default;

    /// Wether parse should actually be parsed directly, before feeding any token to this parser
    /// Usually reserved to empty_parse, but sometimes used when composing several ones too.
    const NOFEED: bool = false;

    /// This method is to bee called whenever feed return true, or if NOFEED=true.
    /// Consume the parser to produce a Self
    ///
    /// It is considered a logic error to call parse if NOFEED is false and `feed` did not return true yet.
    fn parse(p: Self::Parser) -> Result<Self, &'static str>;

    /// The function doing all the hard work.
    /// Take a token, and update the parser from it
    /// If all the desired token were received, return true in order to triger a call to `parse`
    /// If more token are desired, return false
    ///
    /// It is considered a logic error to feed something wich already returned true or
    /// have the NOFEED flag set
    fn feed(tok: Self::Token, parser: &mut Self::Parser) -> Result<bool, &'static str>;
}

/// Trait making it possible to construct Self from some T (&T really),
/// by converting &T into the appropriate Token for Parse-ing (See the Parse trait).
///
/// If you used the macro to implement Parse, this should always be auto-implemented from the relevants &T: TryInto<Self::Token>,
/// meaning you should probably avoid implementing this by hand.
pub trait LexFrom<T: ?Sized>: Parse {
    /// Take a T, wich may be impratical to parse directly, and convert it into a suitable Token to parse Self. (See the Parse trait)
    /// You are given a view on the Parser if the Parser state is needed to know how to parse the token.
    fn lex(t: &T, p: &Self::Parser) -> Result<Self::Token, &'static str>;
}

/// Error typically fired when there is a conversion error while trying to produce a token.
pub const ETYPE: &str = "Wrong type for the next token.";
/// Error fired on end of stream if feed did not yet return true.
pub const EOF: &str = "Unexpected end of input while declaration was unfinished";
/// Error fired on internal error, either from a bug or misuse of the Parse API.
pub const INVALID_STATE: &str = "This parser is in a improper state or was feed an improper token for the attempted operation. \nPlease report a bug or check your usage of the Parse API";

// ***
// # Structs
// ***

/// Data associated to a location.
#[derive(Clone, Copy, Debug)]
pub struct Located<T> {
    pub data: T,
    pub(crate) line: usize,
    pub(crate) toknum: usize,
}

/// The delimiters used to parse lists (Vectors)
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
    /// Convenience function to locate an error at the location of something else
    pub fn err(self, e: &'static str) -> Located<&'static str> {
        Located {
            data: e,
            line: self.line,
            toknum: self.toknum,
        }
    }
}

/// Used to parse Tupples. Avoid using directly
#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub enum Partial<T1: Parse, T2: Parse> {
    None(T1::Parser),
    Fst(T1, T2::Parser),
}

/// Token type for lists of T. Avoid using directly
#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub enum Delimited<T> {
    Delim(Delimiter),
    Val(T),
}

/// Parser type for lists of T. Avoid using directly
#[doc(hidden)]
#[derive(Debug)]
pub struct VecBuilder<T: Parse> {
    parser: Option<T::Parser>,
    delim: bool,
    vec: Vec<T>,
}

/// Parser type for arrays of T. Avoid using directly
#[doc(hidden)]
#[derive(Debug)]
pub struct ArrayParser<T: Parse, const N: usize>(T::Parser, [MaybeUninit<T>; N], usize);

impl<T: Parse, const N: usize> Drop for ArrayParser<T, N> {
    fn drop(&mut self) {
        for elt in self.1.iter_mut().take(self.2) {
            unsafe { elt.assume_init_drop() };
        }
    }
}

// ***
// # Parse implem for classic types
// ***

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
            (parser, _) => (Err(INVALID_STATE), parser),
        })
    }
    const NOFEED: bool = T1::NOFEED && T2::NOFEED;
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
                    Err(INVALID_STATE)
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
                        return Err(INVALID_STATE);
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

impl<T: Parse, const N: usize> Parse for [T; N] {
    type Token = T::Token;
    type Parser = ArrayParser<T, N>;
    const NOFEED: bool = T::NOFEED;
    fn parse(mut p: Self::Parser) -> Result<Self, &'static str> {
        if Self::NOFEED {
            let mut arr: [_; _] = MaybeUninit::uninit().into();
            for i in 0..N {
                match T::parse(T::Parser::default()) {
                    Err(e) => {
                        for old in arr.iter_mut().take(i) {
                            unsafe { old.assume_init_drop() };
                        }

                        return Err(e);
                    }
                    Ok(t) => arr[i].write(t),
                };
            }

            let arr: MaybeUninit<_> = arr.into();
            Ok(unsafe { arr.assume_init() })
        } else {
            if p.2 != N {
                return Err("Array terminated while elements remain to be parsed");
            }
            p.2 = 0; // So Drop do nothing
            let res: MaybeUninit<_> = mem::replace(&mut p.1, MaybeUninit::uninit().into()).into();
            Ok(unsafe { res.assume_init() })
        }
    }
    fn feed(tok: Self::Token, parser: &mut Self::Parser) -> Result<bool, &'static str> {
        match T::feed(tok, &mut parser.0) {
            Err(e) => Err(e),
            Ok(true) => {
                let p = take(&mut parser.0);
                match T::parse(p) {
                    Err(e) => Err(e),
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

// ***
// # Non-trivial LexFrom impls
// ***

impl<T: Parse, D: ?Sized> LexFrom<D> for Vec<T>
where
    T: LexFrom<D>,
    Delimiter: LexFrom<D>,
{
    fn lex(t: &D, p: &Self::Parser) -> Result<Self::Token, &'static str> {
        match (&p.parser, p.delim) {
            (None, false) => Ok(Delimited::Delim(Delimiter::lex(t, &None)?)),
            (None, true) => Err(INVALID_STATE),
            (Some(p), true) => match Delimiter::lex(t, &None) {
                Ok(Delimiter::Close) => Ok(Delimited::Delim(Delimiter::Close)),
                _ => Ok(Delimited::Val(T::lex(t, p)?)),
            },
            (Some(p), false) => Ok(Delimited::Val(T::lex(t, p)?)),
        }
    }
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

// ***
// # Macros
// ***

/// empty_parse!(T) make the type T parsable without consuming any token
/// T must implement Default, and the default value will be returned each time it is parsed
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
                Err($crate::parsing::INVALID_STATE)
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

/// parse_single_tok!(T) make T parsable by lexing to it directly.
/// A long form parse_single_tok!(T,Tok) is also available, to Lex toward a Tok,
/// and the convert to a T using try_into()
#[macro_export]
macro_rules! parse_single_tok {
    ($T: ty) => {
        parse_single_tok!($T,$T);
    };
    ($T: ty,$Tok: ty) => {
        impl $crate::parsing::Parse for $T {
            type Token = $Tok;
            type Parser = core::option::Option<$Tok>;
            fn parse(p: core::option::Option<$Tok>) -> core::result::Result<Self,&'static str> {
                match p.map(|tok| tok.try_into()) {
                    None => Err($crate::parsing::INVALID_STATE),
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
parse_single_tok!(Delimiter);

/// choices!(modname,ResultName,[List]) where List is a comma-separated list of (Name, Type)
/// Create a new Parseable enum, ResultName, which can parse any of the Type, by first recognising wich one
/// by recognising a which Kind it is.
///
/// TODO: WIP:
/// Subject to further proc-macro rewrite to have a better interface
#[macro_export]
macro_rules! choices {
    ($modname:ident,$name:ident,$(($names:ident,$types:ty)),*) => {
     pub mod $modname {
            use super::*;
            use $crate::{parse_single_tok, parsing::{LexFrom, INVALID_STATE, Parse}};

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
                        _ => Err(INVALID_STATE),
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

/// parse_transparent!(T,P) implement Parse for T by recognising a P (which must be Parsable) and then
/// converting the P into a T with try_into() (must be implemented too)
///
/// Especially usefull to parse a combination of Parseable things (into a tupple, array, vec, ...)
/// and combine them into a greater whole.
#[macro_export]
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

// well, two element tupple are ok, but what about 3? 4? more ?
// here it is (all based on the two elements implementation)
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

// **********
// # Boilerplate: Tupple spam, and trivial Default and LexFrom impls
// **********

impl<T1: Parse, T2: Parse> Default for Partial<T1, T2> {
    fn default() -> Self {
        Self::None(Default::default())
    }
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
impl<T: Parse, const N: usize> Default for ArrayParser<T, N> {
    fn default() -> Self {
        Self(Default::default(), MaybeUninit::uninit().into(), 0)
    }
}

impl<S: ?Sized, T: LexFrom<S>, const N: usize> LexFrom<S> for [T; N] {
    fn lex(t: &S, p: &Self::Parser) -> Result<Self::Token, &'static str> {
        T::lex(t, &p.0)
    }
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
