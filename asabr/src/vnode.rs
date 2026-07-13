extern crate alloc;

use alloc::{collections::BTreeMap as HashMap, vec::Vec};

use crate::{
    parse_transparent,
    types::{NodeID, NodeIDMap, NodeName},
};

/// Represents information about a vnode in the network.
#[derive(Debug, Clone)]
pub struct VirtualNodeInfo {
    /// Unique identifier of the virtual node.
    pub vid: NodeID,

    /// Human-readable name of the virtual node.
    pub name: NodeName,

    /// Identifiers of the real nodes represented by this virtual node.
    pub rids: Vec<NodeID>,
}

parse_transparent!(VirtualNodeInfo, (NodeID, NodeName, Vec<NodeID>));

impl From<(NodeID, NodeName, Vec<NodeID>)> for VirtualNodeInfo {
    fn from((vid, name, rids): (NodeID, NodeName, Vec<NodeID>)) -> Self {
        Self { vid, name, rids }
    }
}

/// Bidirectional map between virtual nodes and their associated real nodes.
#[derive(Debug, Default)]
pub struct VirtualNodeMap {
    /// Map from each virtual node ID to the real node IDs it represents.
    pub(crate) vnode_to_rids_map: NodeIDMap,

    /// Map from each real node ID to the virtual node IDs that contain it.
    pub(crate) rid_to_vnodes_map: NodeIDMap,
}

impl VirtualNodeMap {
/// Creates a virtual-node map from both lookup directions.
///
/// The first map associates each virtual node with its real-node members.
/// The second map associates each real node with the virtual nodes containing it.
    pub fn new(
        vnode_to_rids_map: HashMap<NodeID, Vec<NodeID>>,
        rids_to_vnode_map: HashMap<NodeID, Vec<NodeID>>,
    ) -> Self {
        Self {
            vnode_to_rids_map,
            rid_to_vnodes_map: rids_to_vnode_map,
        }
    }

    /// Returns the map from virtual node IDs to their associated real node IDs.
    pub fn get_vnode_to_rids_map(&self) -> &NodeIDMap {
        &self.vnode_to_rids_map
    }

    /// Returns the map from real node IDs to the virtual node IDs containing them.
    pub fn get_rid_to_vnodes_map(&self) -> &NodeIDMap {
        &self.rid_to_vnodes_map
    }

    /// Returns the total number of virtual nodes in the map.
    #[inline(always)]
    pub fn get_vnode_count(&self) -> usize {
        self.vnode_to_rids_map.len()
    }
}
