use crate::{
    bundle::Bundle,
    parsing::{DispatchParser, Lexer, Parser, ParsingState},
    types::Date,
};

use super::NodeManager;

/// Use this manager if no node management shall be considered (with or without the "enable_node_management" compilation feature).
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct NoManagement {}

/// This manager has no effect.
impl NodeManager for NoManagement {
    fn dry_run_process(&self, at_time: Date, _bundle: &mut Bundle) -> Date {
        return at_time;
    }

    fn dry_run_tx(&self, _start: Date, _end: Date, _bundle: &Bundle) -> bool {
        true
    }

    fn dry_run_rx(&self, _start: Date, _end: Date, _bundle: &Bundle) -> bool {
        true
    }

    fn schedule_process(&self, at_time: Date, _bundle: &mut Bundle) -> Date {
        return at_time;
    }

    fn schedule_tx(&mut self, _start: Date, _end: Date, _bundle: &Bundle) -> bool {
        true
    }

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
