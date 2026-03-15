use crate::{
    parsing::{Lexer, Parser, ParsingState},
    types::{NodeID, NodeName, Token},
};

/// Represents information about a vnode in the network.
///
/// # Fields
///
/// * `vid` - The unique identifier for the vnode.
/// * `name` - The name associated with the vnode.
/// * `rids` - A vector of the identifiers of each real node associated with the vnode.
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct VirtualNodeInfo {
    pub vid: NodeID,
    pub name: NodeName,
    pub rids: Vec<NodeID>,
}

impl Parser<VirtualNodeInfo> for VirtualNodeInfo {
    /// Parses a `VirtualNodeInfo` from the provided lexer.
    ///
    /// # Parameters
    ///
    /// * `lexer` - The lexer used to read the vnode information.
    ///
    /// # Returns
    ///
    /// * `ParsingState<VirtualNodeInfo>` - The parsing state, which can be either finished with the parsed node info,
    ///   an error, or an EOF state.
    fn parse(lexer: &mut dyn Lexer) -> ParsingState<VirtualNodeInfo> {
        let vid_state = NodeID::parse(lexer);
        let vid: NodeID = match vid_state {
            ParsingState::Finished(value) => value,
            ParsingState::Error(msg) => return ParsingState::Error(msg),
            ParsingState::EOF => {
                return ParsingState::Error(format!(
                    "Parsing failed ({})",
                    lexer.get_current_position()
                ));
            }
        };

        let name_state = NodeName::parse(lexer);
        let name: NodeName = match name_state {
            ParsingState::Finished(value) => value,
            ParsingState::Error(msg) => return ParsingState::Error(msg),
            ParsingState::EOF => {
                return ParsingState::Error(format!(
                    "Parsing failed ({})",
                    lexer.get_current_position()
                ));
            }
        };

        let rids_state = Vec::parse(lexer);
        let rids: Vec<NodeID> = match rids_state {
            ParsingState::Finished(value) => value,
            ParsingState::Error(msg) => return ParsingState::Error(msg),
            ParsingState::EOF => {
                return ParsingState::Error(format!(
                    "Parsing failed ({})",
                    lexer.get_current_position()
                ));
            }
        };

        ParsingState::Finished(VirtualNodeInfo { vid, name, rids })
    }
}
