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
pub struct HeuristicManagement {
    /// array with the minimal delay from this node to the others
    row: Box<[Date]>,
}

impl HeuristicManagement {
    pub fn new(row: Box<[Date]>) -> Self {
        Self { row }
    }
}

impl NodeManager for HeuristicManagement {
    fn accept(&self, _bundle: &Bundle, _time: TimeInterval, _sender: NodeID) -> bool {
        true
    }

    fn dry_run_retention(
        &self,
        _bundle: &Bundle,
        _reception: TimeInterval,
        _sender: NodeID,
        _transmission: TimeInterval,
        _next: NodeID,
    ) -> bool {
        true
    }

    fn dry_run_multi(
        &self,
        _bundle: &Bundle,
        _reception: TimeInterval,
        _sender: NodeID,
        transmissions: &[(TimeInterval, NodeID)],
    ) -> Option<usize> {
        Some(transmissions.len())
    }

    fn commit(
        &mut self,
        _bundle: &Bundle,
        _reception: TimeInterval,
        _sender: NodeID,
        _transmissions: &[(TimeInterval, NodeID)],
    ) -> Result<(), ASABRError> {
        Ok(())
    }

    fn heuristic_delay_to(&self, target: usize) -> Date {
        self.row.get(target).copied().unwrap_or(0)
    }
}
