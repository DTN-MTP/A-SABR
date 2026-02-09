use std::cell::BorrowMutError;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum ASABRError {
    BorrowMutError(&'static str),
    DryRunError(&'static str),
    ScheduleError(&'static str),
    ContactPlanError(&'static str),
    MulticastUnsupportedError,
}

impl From<BorrowMutError> for ASABRError {
    fn from(_: BorrowMutError) -> Self {
        ASABRError::BorrowMutError("borrow error occurred")
    }
}

impl fmt::Display for ASABRError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for ASABRError {}

impl From<ASABRError> for std::io::Error {
    fn from(err: ASABRError) -> Self {
        std::io::Error::other(err)
    }
}
