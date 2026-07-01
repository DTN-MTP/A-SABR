use crate::bundle::Bundle;
use crate::empty_parse;
use crate::errors::ASABRError;
use crate::types::{NodeID, TimeInterval};

use super::NodeManager;

/// Use this manager if no node management is needed.
#[derive(Debug, Default)]
pub struct NoManagement {}
empty_parse!(NoManagement);

/// This manager has no effect.
impl NodeManager for NoManagement {
    fn accept(&self, _bundle: &Bundle, _time: TimeInterval, _sender: NodeID) -> bool {
        true
    }

    fn dry_run_retention(
        &self,
        _bundle: &Bundle,
        _reception: TimeInterval,
        _sender: NodeID,
        _transmition: TimeInterval,
        _next: NodeID,
    ) -> bool {
        true
    }

    fn dry_run_multi(
        &self,
        _bundle: &Bundle,
        _reception: TimeInterval,
        _sender: NodeID,
        transmitions: &[(TimeInterval, NodeID)],
    ) -> Option<usize> {
        Some(transmitions.len())
    }

    fn commit(
        &mut self,
        _bundle: &Bundle,
        _reception: TimeInterval,
        _sender: NodeID,
        _transmitions: &[(TimeInterval, NodeID)],
    ) -> Result<(), ASABRError> {
        Ok(())
    }
}
