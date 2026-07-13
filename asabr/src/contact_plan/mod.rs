extern crate alloc;
use alloc::vec::Vec;

use crate::contact::Contact;
use crate::contact_manager::ContactManager;
use crate::node::Node;
use crate::node_manager::NodeManager;
use crate::vnode::VirtualNodeInfo;

/// Lexer for the ASABR contact-plan text format.
pub mod asabr_file_lexer;
/// Parser for ASABR lexer tokens.
pub mod from_asabr_lexer;
/// Parser for ION contact-plan files.
pub mod from_ion_file;
/// Parser for TVGUtil contact-plan files.
pub mod from_tvgutil_file;

/// Represents a contact plan and associated management information.
///
///  # Type Parameters
/// - `NNM` and `CNM`: A type implementing the `NodeManager` trait, responsible for managing the
///   node's operations.
/// - `CCM`: A type implementing the `ContactManager` trait, responsible for managing the
///   contact's operations.
#[derive(Clone)]
pub struct ContactPlan<NM: NodeManager, CM: ContactManager> {
    /// Real nodes sorted by ID. `INode`s and `ENode`s.
    pub realnodes: Vec<RealNode<NM>>,
    /// Virtual nodes
    pub vnodes: Vec<VirtualNodeInfo>,
    /// Contacts, sender node, receiver node as index in realnodes
    pub contacts: Vec<(Contact<CM>, usize, usize)>,
}

/// A real node, either internal or external.
#[derive(Clone)]
pub enum RealNode<NM: NodeManager> {
    /// External node.
    Enode(Node<NM>),
    /// Internal node.
    Inode(Node<NM>),
}

impl<NM: NodeManager, CM: ContactManager> ContactPlan<NM, CM> {
    /// Creates a new `ContactPlan`.
    ///
    /// # Parameters
    ///
    /// * `realnodes` - Real nodes stored in node ID order.
    /// * `vnodes` - Virtual-node definitions.
    /// * `contacts` - A vector of contacts that define the connections between nodes.
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
