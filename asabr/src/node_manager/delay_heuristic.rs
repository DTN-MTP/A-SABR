extern crate alloc;
use alloc::boxed::Box;

use crate::bundle::Bundle;
use crate::errors::ASABRError;
use crate::types::{Date, NodeID, TimeInterval};

use super::NodeManager;

/// `NodeManager` that behaves like `NoManagement` except that it has
/// a delay matrix to every other node. Used in `AStar` to compute the
/// delay heuristic.
#[derive(Debug, Clone, Default)]
pub struct DelayHeuristic<T: NodeManager>{
    inner: T,
    /// array with the minimal delay from this node to the others
    row: Box<[Date]>,
}

impl<T: NodeManager> DelayHeuristic<T> {
    pub fn new(inner: T, row: Box<[Date]>) -> Self {
        Self { inner, row }
    }
}

impl<T: NodeManager> NodeManager for DelayHeuristic<T> {
    fn accept(&self, _bundle: &Bundle, _time: TimeInterval, _sender: NodeID) -> bool {
        self.inner.accept(_bundle, _time, _sender)
    }

    fn dry_run_retention(
        &self,
        _bundle: &Bundle,
        _reception: TimeInterval,
        _sender: NodeID,
        _transmission: TimeInterval,
        _next: NodeID,
    ) -> bool {
        self.inner.dry_run_retention(_bundle, _reception, _sender, _transmission, _next)
    }

    fn dry_run_multi(
        &self,
        _bundle: &Bundle,
        _reception: TimeInterval,
        _sender: NodeID,
        transmissions: &[(TimeInterval, NodeID)],
    ) -> Option<usize> {
        self.inner.dry_run_multi(_bundle, _reception, _sender, transmissions)
    }

    fn commit(
        &mut self,
        _bundle: &Bundle,
        _reception: TimeInterval,
        _sender: NodeID,
        _transmissions: &[(TimeInterval, NodeID)],
    ) -> Result<(), ASABRError> {
        self.inner.commit(_bundle, _reception, _sender, _transmissions)
    }

    fn heuristic_delay_to(&self, target: usize) -> Date {
        self.row.get(target).copied().unwrap_or(0)
    }
}
