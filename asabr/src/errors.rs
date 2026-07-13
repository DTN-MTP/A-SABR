use core::cell::{BorrowError, BorrowMutError};
use core::error::Error;
use core::fmt;

use crate::parsing::Located;

#[derive(Debug)]
/// Error type used by A-SABR operations.
pub enum ASABRError {
    /// Returned when a mutable borrow fails.
    BorrowMutError(&'static str),

    /// Returned when a dry-run operation fails.
    DryRunError(&'static str),

    /// Returned when scheduling a bundle or contact fails.
    ScheduleError(&'static str),

    /// Returned when a contact plan cannot be parsed or built correctly.
    ContactPlanError(&'static str),

    /// Returned when multicast routing is requested but not supported.
    MulticastUnsupportedError,

    /// Returned when parsing fails at a specific input location.
    ParsingError(Located<&'static str>),
}

impl From<BorrowError> for ASABRError {
    fn from(_: BorrowError) -> Self {
        ASABRError::BorrowMutError("borrow error occurred")
    }
}

impl From<BorrowMutError> for ASABRError {
    fn from(_: BorrowMutError) -> Self {
        ASABRError::BorrowMutError("mutable borrow error occurred")
    }
}

impl fmt::Display for ASABRError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ASABRError::BorrowMutError(s) => write!(f, "BorrowMutError in A-SABR: {}", s),
            ASABRError::DryRunError(s) => write!(f, "DryRunError in A-SABR: {}", s),
            ASABRError::ScheduleError(s) => write!(f, "ScheduleError in A-SABR: {}", s),
            ASABRError::ContactPlanError(s) => write!(f, "ContactPlanError in A-SABR: {}", s),
            ASABRError::MulticastUnsupportedError => {
                write!(f, "Multicast is Unsupported in A-SABR")
            }
            ASABRError::ParsingError(Located { data, line, toknum }) => write!(
                f,
                "Parsing Error encountered at line {line} token {toknum} in A-SABR: {data}",
            ),
        }
    }
}

impl Error for ASABRError {}
