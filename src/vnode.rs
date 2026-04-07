use std::collections::HashMap;

use crate::{
    parsing::{Lexer, Parser, ParsingState},
    types::{NodeID, NodeIDMap, NodeName, Token},
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

/// Represents a HashMap wich stores virtual node IDs as keys and real node ID lists as values
#[derive(Debug, Default)]
pub struct VirtualNodeMap {
    /// A vnode to nodes NodeIDMap.
    vnode_map: NodeIDMap,
}

impl VirtualNodeMap {
    pub fn new(vnode_map: HashMap<NodeID, Vec<NodeID>>) -> Self {
        Self { vnode_map }
    }

    /// This method does no additional computations and returns reference to the stored NodeIDMap
    pub fn get_vnode_to_rids_map(&self) -> &NodeIDMap {
        &self.vnode_map
    }

    /// This method reverses the HashMap keys and values before returning a corresponding NodeIDMap
    pub fn get_rid_to_vnodes_map(&self) -> NodeIDMap {
        let mut reversed: HashMap<NodeID, Vec<NodeID>> = HashMap::new();

        for (vnode, rids) in &self.vnode_map {
            for rid in rids {
                reversed.entry(*rid).or_default().push(*vnode);
            }
        }

        reversed
    }

    /// Returns the total number of vnodes in the vnode map.
    ///
    /// # Returns
    ///
    /// * `usize` - The total number of nodes.
    pub fn get_vnode_count(&self) -> usize {
        self.vnode_map.len()
    }
}
