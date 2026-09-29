extern crate alloc;
use alloc::boxed::Box;

use crate::bundle::Bundle;
use crate::contact_manager::ContactManager;
use crate::errors::ASABRError;
use crate::multigraph::Multigraph;
use crate::node_manager::NodeHeuristic;
use crate::paths::PathFragment;
use crate::types::{Date, NodeID, TimeInterval};

use super::NodeManager;

/// `NodeManager` that behaves like `NM` except that it has
/// a delay matrix to every other node. Used in `AStar` to compute the
/// delay heuristic.
#[derive(Debug, Clone, Default)]
pub struct DelayHeuristic<NM: NodeManager> {
    inner: NM,
    /// array with the minimal delay from this node to the others
    row: Box<[Date]>,
}

impl<NM: NodeManager> DelayHeuristic<NM> {
    pub fn new(inner: NM, row: Box<[Date]>) -> Self {
        Self { inner, row }
    }
}

impl<NM: NodeManager> NodeManager for DelayHeuristic<NM> {
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
}

impl<NM: NodeManager> NodeHeuristic for DelayHeuristic<NM> {
    fn get_heuristic<'id, CM: ContactManager>(
        path: &PathFragment<'id>,
        graph: &Multigraph<'id, Self, CM>,
        target: NodeID,
    ) -> Date {
        let me = &graph[path.rx_node].manager;
        me.row.get(usize::from(target)).copied().unwrap_or(0)
    }
}
