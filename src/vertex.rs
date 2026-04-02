use std::cmp::Ordering;

use crate::{node::Node, node_manager::NodeManager, types::NodeID};

/// Represents the unique inner identifier of a Vertex in the Multigraph.
pub type VertexID = NodeID;

/// Represents a vertex in the multigraph.
/// In the case of an INode or ENode, this includes its associated manager, and they are wrapped
/// in `Rc<RefCell<...>>` for shared ownership and mutability.
///
/// When sorted, INode and ENode are sorted by inner Node first, then INode < ENode.
/// And VNode is always greatest. It is assumed that vnode IDs are assured to come after real node
/// IDs from the contact plan parser.
///
/// # Type parameters
/// - `NM`: A type implementing the `NodeManager` trait, responsible for managing the
///   node's operations.
#[cfg_attr(feature = "debug", derive(Debug))]
pub enum Vertex<NM: NodeManager> {
    /// An interior node of the graph. Being interior, its contacts point both to it and to its
    /// vnodes at Multigraph creation.
    INode(Node<NM>),
    /// An exterior node of the graph. Being exterior, its contacts only point to its vnodes at
    /// Multigraph creation.
    ENode(Node<NM>),
    /// A virtual node. It is not a node, but an abstraction over one or more node. A "group",
    /// "merger" or "contraction" of nodes.
    /// Thus, it has no manager at all.
    VNode(NodeID),
}

impl<NM: NodeManager> PartialEq for Vertex<NM> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::INode(a), Self::INode(b)) => a == b,
            (Self::ENode(a), Self::ENode(b)) => a == b,
            (Self::VNode(a), Self::VNode(b)) => a == b,
            _ => false,
        }
    }
}

impl<NM: NodeManager> Eq for Vertex<NM> {}

impl<NM: NodeManager> Ord for Vertex<NM> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            // VNodes are always greater
            (Self::VNode(a), Self::VNode(b)) => a.cmp(b),
            (Self::VNode(_), _) => Ordering::Greater,
            (_, Self::VNode(_)) => Ordering::Less,

            // INode and ENode are sorted by inner Node first, then by variant (INode < ENode)
            (Self::INode(a), Self::INode(b)) => a.cmp(b),
            (Self::ENode(a), Self::ENode(b)) => a.cmp(b),
            (Self::INode(a), Self::ENode(b)) => a.cmp(b).then(Ordering::Less),
            (Self::ENode(a), Self::INode(b)) => a.cmp(b).then(Ordering::Greater),
        }
    }
}

impl<NM: NodeManager> PartialOrd for Vertex<NM> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
