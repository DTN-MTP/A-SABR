use std::borrow::Cow;
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

#[derive(Debug)]
pub struct CowError(Cow<'static, str>);

impl CowError {
    /// Borrows `'a str`s, clones and owns `String`s as needed.
    ///
    /// Usage:
    ///
    /// * No allocation for static messages:
    ///   * `return Err(ASABRError::ParsingError(CowError::new("A static error message")));`
    ///
    /// * Allocates and owns a dynamically determined message:
    ///   * `return Err(ASABRError::ParsingError(CowError::new(format!("A dynamic error: {context}"))));`
    pub fn new(msg: impl Into<Cow<'static, str>>) -> Self {
        Self(msg.into())
    }
}

impl fmt::Display for CowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for CowError {}
