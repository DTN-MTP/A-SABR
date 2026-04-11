use crate::contact::Contact;
use crate::contact_manager::ContactManager;
use crate::errors::ASABRError;
use crate::node_manager::NodeManager;
use crate::vertex::Vertex;
use crate::vnode::VirtualNodeMap;

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
    /// Vertices sorted by ID. All `VNode`s come after every `INode`s and `ENode`s.
    pub vertices: Vec<Vertex<NM>>,
    pub contacts: Vec<Contact<NM, CM>>,
    /// Maps vnodes and the nodes they label.
    pub vnode_map: VirtualNodeMap,
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
        vertices: Vec<Vertex<NM>>,
        contacts: Vec<Contact<NM, CM>>,
        vnode_map: Option<VirtualNodeMap>,
    ) -> Result<Self, ASABRError> {
        Ok(ContactPlan {
            vertices,
            contacts,
            vnode_map: vnode_map.unwrap_or_default(),
        })
    }
}
