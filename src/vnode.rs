extern crate alloc;

use alloc::{collections::BTreeMap as HashMap, vec::Vec};

use crate::types::{NodeID, NodeIDMap, NodeName};

/// Represents information about a vnode in the network.
///
/// # Fields
///
/// * `vid` - The unique identifier for the vnode.
/// * `name` - The name associated with the vnode.
/// * `rids` - A vector of the identifiers of each real node associated with the vnode.
#[derive(Debug)]
pub struct VirtualNodeInfo {
    pub vid: NodeID,
    pub name: NodeName,
    pub rids: Vec<NodeID>,
}

pub type VNodeInfoParse = (NodeID, (NodeName, Vec<NodeID>));

impl From<VNodeInfoParse> for VirtualNodeInfo {
    fn from(value: VNodeInfoParse) -> Self {
        let (vid, (name, rids)) = value;
        VirtualNodeInfo { vid, name, rids }
    }
}

/// Represents a HashMap wich stores virtual node IDs as keys and real node ID lists as values
#[derive(Debug, Default)]
pub struct VirtualNodeMap {
    /// A vnode to nodes NodeIDMap.
    vnode_to_rids_map: NodeIDMap,
    rid_to_vnodes_map: NodeIDMap,
}

impl VirtualNodeMap {
    pub fn new(
        vnode_to_rids_map: HashMap<NodeID, Vec<NodeID>>,
        rids_to_vnode_map: HashMap<NodeID, Vec<NodeID>>,
    ) -> Self {
        Self {
            vnode_to_rids_map,
            rid_to_vnodes_map: rids_to_vnode_map,
        }
    }

    /// This method does no additional computations and returns a reference to the stored NodeIDMap
    pub fn get_vnode_to_rids_map(&self) -> &NodeIDMap {
        &self.vnode_to_rids_map
    }

    /// This method does no additional computations and returns a reference to the stored NodeIDMap
    pub fn get_rid_to_vnodes_map(&self) -> &NodeIDMap {
        &self.rid_to_vnodes_map
    }

    /// Returns the total number of vnodes in the vnode map.
    ///
    /// # Returns
    ///
    /// * `usize` - The total number of nodes.
    #[inline(always)]
    pub fn get_vnode_count(&self) -> usize {
        self.vnode_to_rids_map.len()
    }
}
