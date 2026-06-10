extern crate alloc;

use alloc::vec::Vec;

use crate::{
    contact_manager::segmentation::{
        Segment, pseg::PSegmentationManager, seg::SegmentationManager,
    },
    parse_single_tok, parse_transparent,
    types::{DataRate, Duration},
};

#[derive(Clone, Copy, Debug)]
pub enum Token {
    Rate,
    Delay,
}

parse_single_tok!(Token, Token);

impl TryFrom<&str> for Token {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "rate" => Ok(Token::Rate),
            "delay" => Ok(Token::Delay),
            _ => Err(()),
        }
    }
}

pub type SegmentInfo = (Token, Vec<Segment<Duration>>, Token, Vec<Segment<DataRate>>);

impl TryFrom<SegmentInfo> for SegmentationManager {
    type Error = ();
    fn try_from(value: SegmentInfo) -> Result<Self, ()> {
        match value {
            (Token::Delay, delays, Token::Rate, rates)
            | (Token::Rate, rates, Token::Delay, delays) => {
                Ok(SegmentationManager::new(rates, delays))
            }
            _ => Err(()),
        }
    }
}
impl TryFrom<SegmentInfo> for PSegmentationManager {
    type Error = ();
    fn try_from(value: SegmentInfo) -> Result<Self, ()> {
        match value {
            (Token::Delay, delays, Token::Rate, rates)
            | (Token::Rate, rates, Token::Delay, delays) => {
                Ok(PSegmentationManager::new(rates, delays))
            }
            _ => Err(()),
        }
    }
}
parse_transparent!(SegmentationManager, SegmentInfo);
parse_transparent!(PSegmentationManager, SegmentInfo);
