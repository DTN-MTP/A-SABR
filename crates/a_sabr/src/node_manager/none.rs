use a_sabr_macros::{DefaultNodeManager, DefaultNodeRx, DefaultNodeTx};

use crate::node_manager::{NodeRx, NodeTx};
use crate::parsing::{DispatchParser, Lexer, Parser, ParsingState};

use crate::{bundle::Bundle, types::Date};

use super::NodeManager;

/// Use this manager if no node management shall be considered (with or without the node_proc compilation feature).
/// This manager has no effect.
#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(DefaultNodeRx, DefaultNodeTx, DefaultNodeManager)]
pub struct NoManagement {}

/// Implements the DispatchParser to allow dynamic parsing.
impl DispatchParser<NoManagement> for NoManagement {}

/// The parser doesn't need to read tokens.
impl Parser<NoManagement> for NoManagement {
    fn parse(_lexer: &mut dyn Lexer) -> ParsingState<NoManagement> {
        ParsingState::Finished(NoManagement {})
    }
}
