use crate::{
    contact::{Contact, ContactInfo},
    contact_manager::ContactManager,
    contact_plan::ContactPlan,
    node::{Node, NodeInfo},
    parsing::{Parser, StaticMarkerMap},
    types::{NodeID, NodeIDMap, NodeName},
    vnode::{VirtualNodeInfo, VirtualNodeMap},
};
use crate::{
    node_manager::NodeManager,
    parsing::{DispatchParser, Lexer, ParsingState, parse_components},
};
use std::{cmp::max, collections::HashSet};

/// `ContactPlan` is responsible for managing and validating the parsing of contacts and nodes
/// in a network configuration. It tracks known node IDs and names to ensure uniqueness,
/// and verifies that the node IDs match between contacts and nodes.
pub struct ASABRContactPlan {}

impl ASABRContactPlan {
    /// Adds a contact to the contact list, ensuring that the maximum node ID in the contacts is updated.
    ///
    /// # Parameters
    ///
    /// * `contact` - The `Contact` to be added to the plan.
    /// * `contacts` - A mutable reference to a vector of contacts, where the new contact will be stored.
    /// * `max_node_id_in_contacts` - A mutable reference to the current maximum node ID found in contacts.
    ///
    /// # Type Parameters
    ///
    /// * `CM` - A generic type that implements the `ContactManager` trait, used to manage the contact.
    fn add_contact<NM: NodeManager, CM: ContactManager>(
        contact: Contact<NM, CM>,
        contacts: &mut Vec<Contact<NM, CM>>,
        vnode_map: &NodeIDMap,
        max_node_id_in_contacts: &mut usize,
    ) -> Result<(), String> {
        if vnode_map.contains_key(&contact.info.rx_node) {
            return Err(format!(
                "Contact Rx node ({}) cannot be a virtual node",
                contact.info.rx_node
            ));
        } else if vnode_map.contains_key(&contact.info.tx_node) {
            return Err(format!(
                "Contact Tx node ({}) cannot be a virtual node",
                contact.info.tx_node
            ));
        }

        let value = max(contact.get_tx_node(), contact.get_rx_node());
        *max_node_id_in_contacts = max(*max_node_id_in_contacts, value.into());
        contacts.push(contact);
        Ok(())
    }

    /// Adds a node to the node list, ensuring that the node ID and node name are unique.
    /// Returns an error if a node with the same ID or name has already been added.
    ///
    /// # Parameters
    ///
    /// * `node` - The `Node` to be added to the plan.
    /// * `nodes` - A mutable reference to a vector of nodes, where the new node will be stored.
    ///
    /// # Returns
    ///
    /// * `Result<(), String>` - Returns `Ok(())` if the node was successfully added, or an error message
    ///   if there is a conflict with an existing node ID or name.
    ///
    /// # Type Parameters
    ///
    /// * `NM` - A generic type that implements the `NodeManager` trait, used to manage the node.
    fn add_node<NM: NodeManager>(
        node: Node<NM>,
        nodes: &mut Vec<Node<NM>>,
        max_node_id_in_nodes: &mut usize,
        known_node_ids: &mut HashSet<NodeID>,
        known_node_names: &mut HashSet<NodeName>,
    ) -> Result<(), String> {
        let node_id = node.get_node_id();
        let node_name = node.get_node_name();

        if known_node_ids.contains(&node_id) {
            return Err(format!("Two nodes have the same id ({node_id})"));
        }
        if known_node_names.contains(&node_name) {
            return Err(format!("Two nodes have the same name ({node_name})"));
        }
        *max_node_id_in_nodes = max(*max_node_id_in_nodes, node_id.into());
        known_node_ids.insert(node_id);
        known_node_names.insert(node_name);
        nodes.push(node);
        Ok(())
    }

    /// Adds a vnode to the vnode map, performing checks such as bounds on on the vnode IDs.
    /// Returns an error if the checks do not pass.
    ///
    /// # Parameters
    ///
    /// * `vnode` - The `Node` to be added to the plan, constructed from VirtualNodeInfo.
    /// * `vnode_map` - A mutable reference to a NodeIDMap HashMap, where the new vid and rids will be stored.
    /// * `rids` - The vector of NodeIDs to be mapped to this virtual node.
    ///
    /// # Returns
    ///
    /// * `Result<(), String>` - Returns `Ok(())` if the node was successfully added, or an error message
    ///   if any of the error control checks fail.
    fn add_vnode(
        vnode_info: VirtualNodeInfo,
        vnode_map: &mut NodeIDMap,
        known_node_ids: &HashSet<NodeID>,
        max_node_id_in_nodes: &mut usize,
    ) -> Result<(), String> {
        // Error control checks
        // 1. A vnode ID must come after all the real node IDs it references.

        // `max_node_id_in_nodes` is #nodes + #vnodes including the current one.
        // `vnode_map.len()` hasn't been updated yet so it excludes the current one.
        // Thus:
        let max_real_node_id = (*max_node_id_in_nodes - 1) - vnode_map.len();

        if usize::from(vnode_info.vid) <= max_real_node_id {
            return Err(format!(
                "Virtual node ID is in the range of its real node IDs (vid {} <= {})",
                vnode_info.vid, max_real_node_id
            ));
        }

        // 2. A vnode's rids mut be in range and must not be duplicates.
        let mut known_rids: HashSet<NodeID> = HashSet::new();
        for rid in &vnode_info.rids {
            if usize::from(*rid) > max_real_node_id {
                return Err(format!(
                    "Node ID out of range ({rid} > {max_real_node_id}) in virtual node definition"
                ));
            }
            if !known_node_ids.contains(rid) {
                return Err(format!(
                    "Node ID referenced in in virtual node definition does not exist ({rid})"
                ));
            }
            if known_rids.contains(rid) {
                return Err(format!(
                    "Node ID duplicate ({rid}) in virtual node definition"
                ));
            }
            known_rids.insert(*rid);
        }

        vnode_map.insert(vnode_info.vid, vnode_info.rids);

        Ok(())
    }

    /// Parses nodes and contacts from a lexer, while ensuring node ID and name uniqueness
    /// and consistency between node definitions and contacts.
    ///
    /// The lexer processes tokens from input text, and this method associates each parsed element
    /// with a node or a contact. It uses marker maps to recognize elements based on predefined markers.
    /// Do not provide the associated marker map if you plan to use a dyn NodeManager or dyn ContactManager.
    ///
    /// # Parameters
    ///
    /// * `lexer` - A mutable reference to a `Lexer` instance, which provides tokens from the input text.
    /// * `node_marker_map` - An optional hash map that associates node markers with parsing functions.
    /// * `contact_marker_map` - An optional hash map that associates contact markers with parsing functions.
    ///
    /// # Returns
    ///
    /// * `Result<ContactPlan<NM, NM, CM>, String>` - Returns a `ContactPlan` containing vectors of parsed
    ///   nodes and contacts, or an error message if there is an issue during parsing.
    ///
    /// # Type Parameters
    ///
    /// * `NM` - A type that implements the `NodeManager`, Parser<NM>, and `DispatchParser<NM>` traits, representing
    ///   the type of the nodes being managed and parsed.
    /// * `CM` - A type that implements the `ContactManager`, Parser<CM>, and `DispatchParser<CM>` traits, representing
    ///   the type of the contacts being managed and parsed.
    pub fn parse<
        NM: NodeManager + DispatchParser<NM> + Parser<NM>,
        CM: ContactManager + DispatchParser<CM> + Parser<CM>,
    >(
        lexer: &mut dyn Lexer,
        node_marker_map: Option<&StaticMarkerMap<NM>>,
        contact_marker_map: Option<&StaticMarkerMap<CM>>,
    ) -> Result<ContactPlan<NM, CM>, String> {
        let mut contacts: Vec<Contact<NM, CM>> = Vec::new();
        let mut nodes: Vec<Node<NM>> = Vec::new();
        let mut vnode_map: NodeIDMap = NodeIDMap::new();

        // These include nodes and vnodes
        let mut known_node_ids: HashSet<NodeID> = HashSet::new();
        let mut known_node_names: HashSet<NodeName> = HashSet::new();
        let mut max_node_id_in_contacts: usize = 0;
        let mut max_node_id_in_nodes: usize = 0;

        loop {
            let res = lexer.consume_next_token();

            match res {
                ParsingState::EOF => {
                    break;
                }
                ParsingState::Error(msg) => {
                    return Err(msg);
                }
                ParsingState::Finished(element_type) => match element_type.as_str() {
                    "contact" => {
                        let contact =
                            parse_components::<ContactInfo, CM>(lexer, contact_marker_map);
                        match contact {
                            ParsingState::EOF => {
                                break;
                            }
                            ParsingState::Error(msg) => {
                                return Err(msg);
                            }
                            ParsingState::Finished((info, manager)) => {
                                let Some(contact) = Contact::try_new(info, manager) else {
                                    return Err(format!(
                                        "Malformed contact ({})",
                                        lexer.get_current_position()
                                    ));
                                };

                                Self::add_contact(
                                    contact,
                                    &mut contacts,
                                    &vnode_map,
                                    &mut max_node_id_in_contacts,
                                )?;
                            }
                        }
                    }
                    "node" => {
                        let node = parse_components::<NodeInfo, NM>(lexer, node_marker_map);
                        match node {
                            ParsingState::EOF => {
                                break;
                            }
                            ParsingState::Error(msg) => {
                                return Err(msg);
                            }
                            ParsingState::Finished((info, manager)) => {
                                let Some(node) = Node::try_new(info, manager) else {
                                    return Err(format!(
                                        "Malformed node ({})",
                                        lexer.get_current_position()
                                    ));
                                };

                                Self::add_node(
                                    node,
                                    &mut nodes,
                                    &mut max_node_id_in_nodes,
                                    &mut known_node_ids,
                                    &mut known_node_names,
                                )?;
                            }
                        }
                    }
                    "vnode" => {
                        let vnode = parse_components::<VirtualNodeInfo, NM>(lexer, node_marker_map);
                        match vnode {
                            ParsingState::EOF => {
                                break;
                            }
                            ParsingState::Error(msg) => {
                                return Err(msg);
                            }
                            ParsingState::Finished((info, manager)) => {
                                // A vnode is not only a mapping to a list of NodeIDs in a vnode_map, it is also a real node in the graph.
                                // Thus here we instantiate a Node object from the VirtualNodeInfo
                                let node_info = NodeInfo {
                                    id: info.vid,
                                    name: info.name.clone(),
                                    excluded: false,
                                };

                                let Some(vnode) = Node::try_new(node_info, manager) else {
                                    return Err(format!(
                                        "Malformed node ({})",
                                        lexer.get_current_position()
                                    ));
                                };

                                // Add the vnode to the nodes list, returning on error.
                                // This also updates max_node_id_in_nodes, known_node_ids and
                                // known_node_names.
                                Self::add_node(
                                    vnode,
                                    &mut nodes,
                                    &mut max_node_id_in_nodes,
                                    &mut known_node_ids,
                                    &mut known_node_names,
                                )?;

                                // Add the vnode to the vnode_map, returning on error.
                                // add_vnode must come after add_node, as it relies on previous
                                // checks and updates to the nodes list.
                                Self::add_vnode(
                                    info,
                                    &mut vnode_map,
                                    &known_node_ids,
                                    &mut max_node_id_in_nodes,
                                )?;
                            }
                        }
                    }
                    _ => {
                        return Err(format!(
                            "Unrecognized CP element ({})",
                            lexer.get_current_position()
                        ));
                    }
                },
            }
        }
        if vnode_map.is_empty() && (max_node_id_in_contacts != max_node_id_in_nodes) {
            return Err(
                "The max node numbers for the contact and node definitions do not match"
                    .to_string(),
            );
        }
        if nodes.is_empty() {
            return Err("Nodes must be declared".to_string());
        }
        if nodes.len() - 1 != max_node_id_in_nodes {
            return Err("Some node declarations are missing".to_string());
        }

        ContactPlan::new(nodes, contacts, Some(VirtualNodeMap::new(vnode_map)))
            .map_err(|_| "Failed to create contact plan".to_string())
    }
}
