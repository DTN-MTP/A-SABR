extern crate alloc;

use crate::{
    node::Node,
    node_manager::NodeManager,
    types::{NodeID, NodeName},
};

/// Represents the unique inner identifier of a Vertex in the Multigraph.
pub type VertexID = NodeID;

pub type VNode = (NodeName, NodeID);

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
#[derive(Debug)]
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
    VNode(VNode),
}
