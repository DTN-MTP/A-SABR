use core::cell::{BorrowError, BorrowMutError};
use core::error::Error;
use core::fmt;

#[derive(Debug)]
pub enum ASABRError {
    BorrowMutError(&'static str),
    DryRunError(&'static str),
    ScheduleError(&'static str),
    ContactPlanError(&'static str),
    MulticastUnsupportedError,
    ParsingError(&'static str, (usize, usize)),
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
            ASABRError::ParsingError(s, l) => write!(
                f,
                "Parsing Error encountered at line {} tocken {} in A-SABR: {}",
                l.0, l.1, s
            ),
        }
    }
}

impl Error for ASABRError {}

//     fn from(err: ASABRError) -> Self {
//         std::io::Error::other(err)
//     }
// }
