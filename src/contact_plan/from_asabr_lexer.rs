use crate::{
    contact::{Contact, ContactInfo},
    contact_manager::ContactManager,
    node::{Node, NodeInfo},
    parsing::{Dispatcher, Parser},
    types::{NodeID, NodeName},
};
use crate::{
    node_manager::NodeManager,
    parsing::{parse_components, DispatchParser, Lexer, ParsingState},
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
    ///
    /// # Type Parameters
    ///
    /// * `CM` - A generic type that implements the `ContactManager` trait, used to manage the contact.
    fn add_contact<NM: NodeManager, CM: ContactManager>(
        contact: Contact<NM, CM>,
        contacts: &mut Vec<Contact<NM, CM>>,
        max_node_id_in_contacts: &mut usize,
    ) {
        let value = max(contact.get_tx_node(), contact.get_rx_node());
        *max_node_id_in_contacts = max(*max_node_id_in_contacts, value.into());
        contacts.push(contact);
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
        max_node_in_in_nodes: &mut usize,
        known_node_ids: &mut HashSet<NodeID>,
        known_node_names: &mut HashSet<NodeName>,
    ) -> Result<(), String> {
        let node_id = node.get_node_id();
        let node_name = node.get_node_name();

        if known_node_ids.contains(&node_id) {
            return Err(format!("Two nodes have the same id ({})", node_id));
        }
        if known_node_names.contains(&node_name) {
            return Err(format!("Two nodes have the same id ({})", node_name));
        }
        let value = max(node.get_node_id(), node.get_node_id());
        *max_node_in_in_nodes = max(*max_node_in_in_nodes, value.into());
        known_node_ids.insert(node_id);
        known_node_names.insert(node_name);
        nodes.push(node);
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
    /// * `Result<(Vec<Node<NM>>, Vec<Contact<NM, CM>>), String>` - Returns a tuple containing vectors of parsed
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
        node_marker_map: Option<&Dispatcher<fn(&mut dyn Lexer) -> ParsingState<NM>>>,
        contact_marker_map: Option<&Dispatcher<fn(&mut dyn Lexer) -> ParsingState<CM>>>,
    ) -> Result<(Vec<Node<NM>>, Vec<Contact<NM, CM>>), String> {
        let mut contacts: Vec<Contact<NM, CM>> = Vec::new();
        let mut nodes: Vec<Node<NM>> = Vec::new();

        let mut known_node_ids: HashSet<NodeID> = HashSet::new();
        let mut known_node_names: HashSet<NodeName> = HashSet::new();
        let mut max_node_id_in_contacts: usize = 0;
        let mut max_node_in_in_nodes: usize = 0;

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
                                if let Some(contact) = Contact::try_new(info, manager) {
                                    Self::add_contact(
                                        contact,
                                        &mut contacts,
                                        &mut max_node_id_in_contacts,
                                    );
                                } else {
                                    return Err(format!(
                                        "Malformed contact ({})",
                                        lexer.get_current_position()
                                    ));
                                }
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
                                if let Some(node) = Node::try_new(info, manager) {
                                    match Self::add_node(
                                        node,
                                        &mut nodes,
                                        &mut max_node_in_in_nodes,
                                        &mut known_node_ids,
                                        &mut known_node_names,
                                    ) {
                                        Ok(_) => {}
                                        Err(msg) => {
                                            return Err(msg);
                                        }
                                    }
                                } else {
                                    return Err(format!(
                                        "Malformed node ({})",
                                        lexer.get_current_position()
                                    ));
                                }
                            }
                        }
                    }
                    _ => {
                        return Err(format!(
                            "Unrecognized CP element ({})",
                            lexer.get_current_position()
                        ))
                    }
                },
            }
        }
        if max_node_id_in_contacts != max_node_in_in_nodes {
            return Err(
                "The max node numbers for the contact and node definitions do not match"
                    .to_string(),
            );
        }
        if nodes.is_empty() {
            return Err("Nodes must be declared".to_string());
        }
        if nodes.len() - 1 != max_node_id_in_contacts {
            return Err("Some node declarations are missing".to_string());
        }
        Ok((nodes, contacts))
    }
}
