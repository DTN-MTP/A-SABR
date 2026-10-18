extern crate alloc;

use alloc::vec::Vec;

use crate::{
    contact_manager::segmentation::{
        Segment, poly_seg::PolySegManager, pseg::PSegmentationManager, seg::SegmentationManager,
    },
    parse_single_tok, parse_transparent,
    poly::Polynome,
    types::{DataRate, Date, Duration},
};

/// Tokens used to identify segmentation fields.
#[derive(Clone, Copy, Debug)]
pub enum Token {
    /// Data-rate section.
    Rate,
    /// Delay section.
    Delay,
    /// Polynomial section.
    Poly,
}

parse_single_tok!(Token, Token);

impl TryFrom<&str> for Token {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "rate" => Ok(Token::Rate),
            "delay" => Ok(Token::Delay),
            "poly" => Ok(Token::Poly),
            _ => Err(()),
        }
    }
}

/// Parsed segmentation manager data.
pub type SegmentInfo = (Token, Vec<Segment<Duration>>, Token, Vec<Segment<DataRate>>);

pub const MAX_N: usize = 26;

pub type PolyMax = Polynome<MAX_N>;
parse_single_tok!(PolyMax);

/// Parsed polynomial segmentation manager data.
pub type PolySegmentInfo = (Token, Vec<Segment<Duration>>, Token, Vec<i64>, Date);

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

impl TryFrom<PolySegmentInfo> for PolySegManager<MAX_N> {
    type Error = ();
    fn try_from(value: PolySegmentInfo) -> Result<Self, ()> {
        match value {
            (Token::Delay, delays, Token::Poly, coeffs_vec, offset) => {
                let mut coeffs = [0; MAX_N];
                for (i, c) in coeffs_vec.into_iter().enumerate() {
                    if i < MAX_N {
                        coeffs[i] = c;
                    }
                }

                let poly = Polynome::new(coeffs, offset);
                Ok(PolySegManager::new(poly, delays))
            }
            _ => Err(()),
        }
    }
}
parse_transparent!(SegmentationManager, SegmentInfo);
parse_transparent!(PSegmentationManager, SegmentInfo);
parse_transparent!(PolySegManager<MAX_N>, PolySegmentInfo);
