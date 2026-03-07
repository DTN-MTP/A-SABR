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

pub struct ContactPlan<NNM: NodeManager, CNM: NodeManager, CCM: ContactManager> {
    pub nodes: Vec<Node<NNM>>,
    pub contacts: Vec<Contact<CNM, CCM>>,
    pub vnode_map: VirtualNodeMap,
}

impl<NNM: NodeManager, CNM: NodeManager, CCM: ContactManager> ContactPlan<NNM, CNM, CCM> {
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
