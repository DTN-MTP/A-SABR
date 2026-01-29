use std::cell::BorrowMutError;

pub enum ASABRError {
    BorrowMutError(&'static str),
    DryRun(&'static str),
    ScheduleError(&'static str),
    MulticastUnsupported,
}

impl From<BorrowMutError> for ASABRError {
    fn from(_: BorrowMutError) -> Self {
        ASABRError::BorrowMutError("borrow error occurred")
    }
}
