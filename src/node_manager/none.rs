use crate::parsing::{DispatchParser, Lexer, Parser, ParsingState};

#[cfg(any(feature = "node_proc", feature = "node_tx", feature = "node_rx"))]
use crate::{bundle::Bundle, types::Date};

use super::NodeManager;

/// Use this manager if no node management shall be considered (with or without the node_rx, node_tx, and node_proc compilation feature).
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct NoManagement {}

/// This manager has no effect.
impl NodeManager for NoManagement {
    #[cfg(feature = "node_proc")]
    fn dry_run_process(&self, at_time: Date, _bundle: &mut Bundle) -> Date {
        return at_time;
    }
    #[cfg(feature = "node_tx")]
    fn dry_run_tx(&self, _waiting_since: Date, _start: Date, _end: Date, _bundle: &Bundle) -> bool {
        true
    }
    #[cfg(feature = "node_rx")]
    fn dry_run_rx(&self, _start: Date, _end: Date, _bundle: &Bundle) -> bool {
        true
    }
    #[cfg(feature = "node_proc")]
    fn schedule_process(&self, at_time: Date, _bundle: &mut Bundle) -> Date {
        return at_time;
    }
    #[cfg(feature = "node_tx")]
    fn schedule_tx(
        &mut self,
        _waiting_since: Date,
        _start: Date,
        _end: Date,
        _bundle: &Bundle,
    ) -> bool {
        true
    }
    #[cfg(feature = "node_rx")]
    fn schedule_rx(&mut self, _start: Date, _end: Date, _bundle: &Bundle) -> bool {
        true
    }
}

/// Implements the DispatchParser to allow dynamic parsing.
impl DispatchParser<NoManagement> for NoManagement {}

/// The parser doesn't need to read tokens.
impl Parser<NoManagement> for NoManagement {
    fn parse(_lexer: &mut dyn Lexer) -> ParsingState<NoManagement> {
        ParsingState::Finished(NoManagement {})
    }
}
