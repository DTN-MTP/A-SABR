use crate::contact::Contact;
use crate::contact_manager::ContactManager;
use crate::errors::ASABRError;
use crate::node::Node;
use crate::node_manager::NodeManager;
use crate::types::VirtualNodeMap;

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
pub struct ContactPlan<NNM: NodeManager, CNM: NodeManager, CCM: ContactManager> {
    pub nodes: Vec<Node<NNM>>,
    pub contacts: Vec<Contact<CNM, CCM>>,
    pub vnode_map: VirtualNodeMap,
}

impl<NNM: NodeManager, CNM: NodeManager, CCM: ContactManager> ContactPlan<NNM, CNM, CCM> {
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
    fn new(
        nodes: Vec<Node<NNM>>,
        contacts: Vec<Contact<CNM, CCM>>,
        vnode_map: Option<VirtualNodeMap>,
    ) -> Result<ContactPlan<NNM, CNM, CCM>, ASABRError> {
        Ok(ContactPlan {
            nodes,
            contacts,
            vnode_map: vnode_map.unwrap_or_default(),
        })
    }
}
