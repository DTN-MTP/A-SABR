use std::collections::HashMap;

use crate::{
    errors::ASABRError,
    parsing::{Lexer, Parser},
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
    /// * `Result<LexerOutput<VirtualNodeInfo>, ASABRError>` - The parsing state, which can be either
    ///   finished with the parsed node info, an error, or an EOF state.
    fn parse(lexer: &mut dyn Lexer) -> Result<VirtualNodeInfo, ASABRError> {
        let vid: NodeID = NodeID::parse(lexer)?;

        let name: NodeName = NodeName::parse(lexer)?;

        let rids: Vec<NodeID> = Vec::parse(lexer)?;
        for i in 0..rids.len() {
            for j in (i + 1)..rids.len() {
                if rids[i] == rids[j] {
                    return Err(ASABRError::ParsingError(format!(
                        "Parsing failed: duplicate node ID in vnode definition ({})",
                        lexer.get_current_position()
                    )));
                }
            }
        }

        Ok(VirtualNodeInfo { vid, name, rids })
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

    /// This method does no additional computations and returns a reference to the stored NodeIDMap
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
    #[inline(always)]
    pub fn get_vnode_count(&self) -> usize {
        self.vnode_map.len()
    }
}
