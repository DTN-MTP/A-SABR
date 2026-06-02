extern crate alloc;

use alloc::vec::Vec;

use crate::{
    choices,
    contact_manager::segmentation::{
        Segment, SegmentParse, pseg::PSegmentationManager, seg::SegmentationManager,
    },
    parse_transparent,
    parsing::{Delimited, Delimiter, LexFrom, Parse},
    types::{DataRate, Duration},
};

choices!(
    infos,
    Segments,
    (Rate, SegmentParse<DataRate>),
    (Delay, SegmentParse<Duration>)
);

pub use infos::{Kinds as SegmentsKind, Segments};

impl<'a> TryFrom<&'a str> for SegmentsKind {
    type Error = ();
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        match value {
            "rate" => Ok(Self::Rate),
            "delay" => Ok(Self::Delay),
            _ => Err(()),
        }
    }
}

pub struct SegmentInfo {
    pub delays: Vec<Segment<Duration>>,
    pub rates: Vec<Segment<DataRate>>,
}

impl Parse for SegmentInfo {
    type Token = Delimited<infos::Token>;
    type Parser = (
        Option<infos::Parser>,
        Vec<Segment<Duration>>,
        Vec<Segment<DataRate>>,
    );
    fn parse(p: Self::Parser) -> Result<Self, &'static str> {
        let (p, delays, rates) = p;
        match p {
            Some(_) => panic!("{p:?},{delays:?},{rates:?}"),
            // Err("Last segment is not finished"),
            None => Ok(SegmentInfo { delays, rates }),
        }
    }
    fn feed(tok: Self::Token, parser: &mut Self::Parser) -> Result<bool, &'static str> {
        match (tok, &mut parser.0) {
            (Delimited::Delim(_), Some(_)) => Err("Last segment is not finished"),
            (Delimited::Delim(Delimiter::Open), None) => Ok(false),
            (Delimited::Delim(Delimiter::Close), None) => Ok(true),
            (Delimited::Val(tok), Some(sub)) => {
                if Segments::feed(tok, sub)? {
                    //unwrap is Ok, we just matched it with Some(sub)
                    match Segments::parse(parser.0.take().unwrap())? {
                        Segments::Rate(seg) => {
                            parser.1.push(seg.into());
                        }
                        Segments::Delay(seg) => {
                            parser.2.push(seg.into());
                        }
                    }
                }
                Ok(false)
            }
            (Delimited::Val(tok), None) => {
                let mut new = Default::default();
                if Segments::feed(tok, &mut new)? {
                    match Segments::parse(new)? {
                        Segments::Rate(seg) => {
                            parser.1.push(seg.into());
                        }
                        Segments::Delay(seg) => {
                            parser.2.push(seg.into());
                        }
                    }
                } else {
                    parser.0 = Some(new);
                }
                Ok(false)
            }
        }
    }
}

impl<T: ?Sized> LexFrom<T> for SegmentInfo
where
    Delimiter: LexFrom<T>,
    Segments: LexFrom<T>,
{
    fn lex(t: &T, p: &Self::Parser) -> Result<Self::Token, &'static str> {
        match Delimiter::lex(t, &None) {
            Ok(delim) => Ok(Delimited::Delim(delim)),
            Err(_) => match &p.0 {
                None => {
                    let new = Default::default();
                    Ok(Delimited::Val(Segments::lex(t, &new)?))
                }
                Some(p) => Ok(Delimited::Val(Segments::lex(t, p)?)),
            },
        }
    }
}

impl From<SegmentInfo> for SegmentationManager {
    fn from(value: SegmentInfo) -> Self {
        SegmentationManager::new(value.rates, value.delays)
    }
}
impl From<SegmentInfo> for PSegmentationManager {
    fn from(value: SegmentInfo) -> Self {
        PSegmentationManager::new(value.rates, value.delays)
    }
}
parse_transparent!(SegmentationManager, SegmentInfo);
parse_transparent!(PSegmentationManager, SegmentInfo);
