extern crate alloc;
use alloc::vec::Vec;

use crate::contact::Contact;
use crate::contact_manager::ContactManager;
use crate::node::Node;
use crate::node_manager::NodeManager;
use crate::vnode::VirtualNodeInfo;

pub mod asabr_file_lexer;
pub mod from_asabr_lexer;
pub mod from_ion_file;
pub mod from_tvgutil_file;

/// Represents a contact plan and associated management information.
///
///  # Type Parameters
/// - `NNM` and `CNM`: A type implementing the `NodeManager` trait, responsible for managing the
///   node's operations.
/// - `CCM`: A type implementing the `ContactManager` trait, responsible for managing the
///   contact's operations.
pub struct ContactPlan<NM: NodeManager, CM: ContactManager> {
    /// Real nodes sorted by ID. `INode`s and `ENode`s.
    pub(crate) realnodes: Vec<RealNode<NM>>,
    /// Virtual nodes
    pub(crate) vnodes: Vec<VirtualNodeInfo>,
    /// Contacts, sender node, receiver node as index in realnodes
    pub(crate) contacts: Vec<(Contact<CM>, usize, usize)>,
}

pub enum RealNode<NM: NodeManager> {
    Enode(Node<NM>),
    Inode(Node<NM>),
}

impl<NM: NodeManager, CM: ContactManager> ContactPlan<NM, CM> {
    /// Creates a new `ContactPlan`.
    ///
    /// # Parameters
    ///
    /// * `nodes` - A vector of nodes
    /// * `contacts` - A vector of contacts that define the connections between nodes.
    /// * `vnode_map` - A HashMap wich stores virtual node IDs as keys and real node ID lists as values
    ///
    /// # Returns
    ///
    /// * `Self` - A new instance of `ContactPlan`.
    pub fn new(
        realnodes: Vec<RealNode<NM>>,
        vnodes: Vec<VirtualNodeInfo>,
        contacts: Vec<(Contact<CM>, usize, usize)>,
    ) -> Self {
        Self {
            realnodes,
            vnodes,
            contacts,
        }
    }
}
