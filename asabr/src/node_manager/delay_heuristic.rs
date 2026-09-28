extern crate alloc;
use alloc::boxed::Box;

use crate::bundle::Bundle;
use crate::errors::ASABRError;
use crate::types::{Date, HeuristicQuery, NodeID, TimeInterval};

use super::NodeManager;

/// `NodeManager` that behaves like `T` except that it has
/// a delay matrix to every other node. Used in `AStar` to compute the
/// delay heuristic.
#[derive(Debug, Clone, Default)]
pub struct DelayHeuristic<T: NodeManager> {
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
    fn accept(&self, bundle: &Bundle, time: TimeInterval, sender: NodeID) -> bool {
        self.inner.accept(bundle, time, sender)
    }

    fn process_delay(
        &self,
        bundle: &Bundle,
        reception: TimeInterval,
        sender: NodeID,
        nextvertex: NodeID,
    ) -> Date {
        self.inner
            .process_delay(bundle, reception, sender, nextvertex)
    }

    fn dry_run_retention(
        &self,
        bundle: &Bundle,
        reception: TimeInterval,
        sender: NodeID,
        transmission: TimeInterval,
        next: NodeID,
    ) -> bool {
        self.inner
            .dry_run_retention(bundle, reception, sender, transmission, next)
    }

    fn dry_run_multi(
        &self,
        bundle: &Bundle,
        reception: TimeInterval,
        sender: NodeID,
        transmissions: &[(TimeInterval, NodeID)],
    ) -> Option<usize> {
        self.inner
            .dry_run_multi(bundle, reception, sender, transmissions)
    }

    fn commit(
        &mut self,
        bundle: &Bundle,
        reception: TimeInterval,
        sender: NodeID,
        transmissions: &[(TimeInterval, NodeID)],
    ) -> Result<(), ASABRError> {
        self.inner.commit(bundle, reception, sender, transmissions)
    }

    fn heuristic_delay(&self, query: &HeuristicQuery) -> Date {
        let own_heuristic = self
            .row
            .get(usize::from(query.target))
            .copied()
            .unwrap_or(0);

        own_heuristic.max(self.inner.heuristic_delay(query))
    }
}
