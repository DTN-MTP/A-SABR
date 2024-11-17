use std::cell::RefCell;
use std::rc::Rc;

use super::node::Node;
use crate::contact::Contact;
use crate::contact_manager::ContactManager;
use crate::node_manager::NodeManager;
use crate::types::*;

/// Represents a sender node in a routing system, with associated receivers.
///
/// The `Sender` struct holds a reference to a sender node and a list of `Receiver`
/// instances that represent the intended recipients for messages or routing actions.
///
/// # Generic Parameters
/// - `NM`: A type implementing the `NodeManager` trait, responsible for managing node-level operations.
/// - `CM`: A type implementing the `ContactManager` trait, responsible for managing contact-level operations.
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Sender<NM: NodeManager, CM: ContactManager> {
    /// The node represented by this sender, wrapped in `Rc<RefCell<...>>` for shared ownership and mutability.
    pub node: Rc<RefCell<Node<NM>>>,
    /// A list of receivers that this sender can communicate with or send data to.
    pub receivers: Vec<Receiver<NM, CM>>,
}

/// Represents a receiver node, along with its contacts and routing information.
///
/// The `Receiver` struct holds references to contacts that provide paths to this receiver,
/// and it also includes a mechanism for lazy pruning of outdated contacts based on a time threshold.
///
/// # Generic Parameters
/// - `NM`: A type implementing the `NodeManager` trait, managing node-level operations.
/// - `CM`: A type implementing the `ContactManager` trait, managing contact-level operations.
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Receiver<NM: NodeManager, CM: ContactManager> {
    /// The node represented by this receiver, wrapped in `Rc<RefCell<...>>`.
    pub node: Rc<RefCell<Node<NM>>>,
    /// A list of contacts providing paths to this receiver.
    pub contacts_to_receiver: Vec<Rc<RefCell<Contact<CM>>>>,
    /// The index of the next contact to be checked for relevance.
    pub next: usize,
}

impl<NM: NodeManager, CM: ContactManager> Receiver<NM, CM> {
    /// Lazily prunes outdated contacts and returns the index of the first valid contact.
    ///
    /// This method iterates over `contacts_to_receiver`, starting from the index stored in `self.next`.
    /// It checks if each contact is still valid based on its expiration time. Once a valid contact
    /// is found, it updates `self.next` and returns the index of this contact.
    ///
    /// # Parameters
    /// - `current_time`: The current time against which contact expiration is checked.
    ///
    /// # Returns
    /// - `Some(usize)`: The index of the first valid contact if found.
    /// - `None`: If no valid contact is found.
    pub fn lazy_prune_and_get_first_idx(&mut self, current_time: Date) -> Option<usize> {
        for (idx, contact) in self.contacts_to_receiver.iter().enumerate().skip(self.next) {
            if contact.borrow().info.end > current_time {
                self.next = idx;
                return Some(idx);
            }
        }
        return None;
    }

    /// Checks if the receiver's node is excluded from routing or pathfinding.
    ///
    /// This method provides a quick check on whether the receiver node is excluded
    /// from any routing operations. This is useful for selectively excluding nodes
    /// without removing them from the network entirely.
    ///
    /// # Returns
    /// - `true`: If the receiver node is excluded.
    /// - `false`: If the receiver node is included.
    pub fn is_excluded(&self) -> bool {
        return self.node.borrow().info.excluded;
    }
}

/// Represents a multigraph structure, where each node can have multiple connections.
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Multigraph<NM: NodeManager, CM: ContactManager> {
    /// * `senders` - The list of sender objects.
    pub senders: Vec<Sender<NM, CM>>,
    /// * `nodes` - The list of node objects.
    pub nodes: Vec<Rc<RefCell<Node<NM>>>>,
    /// * `node_count` - The total number of nodes in the multigraph.
    node_count: usize,
}

impl<NM: NodeManager, CM: ContactManager> Multigraph<NM, CM> {
    /// Creates a new `Multigraph` from a list of nodes and a contact plan.
    ///
    /// # Parameters
    ///
    /// * `nodes` - A vector of nodes to be included in the multigraph.
    /// * `contact_plan` - A vector of contacts that define the connections between nodes.
    ///
    /// # Returns
    ///
    /// * `Self` - A new instance of `Multigraph`.
    pub fn new(mut nodes: Vec<Node<NM>>, mut contact_plan: Vec<Contact<CM>>) -> Self {
        // the contact plan might not be sorted
        // having a sorted list of contacts allow easy multigraph creation
        let mut data: Vec<Sender<NM, CM>> = Vec::new();
        let node_count = nodes.len();

        contact_plan.sort_unstable();
        nodes.sort_unstable();
        nodes.reverse();

        let mut all_refs = Vec::new();

        while let Some(node) = nodes.pop() {
            let node_ref = Rc::new(RefCell::new(node));
            data.push(Sender {
                node: Rc::clone(&node_ref),
                receivers: Vec::new(),
            });
            all_refs.push(node_ref)
        }

        for sender in &mut data {
            for node_ref in &all_refs {
                sender.receivers.push(Receiver {
                    node: Rc::clone(node_ref),
                    contacts_to_receiver: Vec::new(),
                    next: 0,
                });
            }
        }

        for contact in contact_plan {
            let tx_node_id: NodeID = contact.get_tx_node();
            let rx_node_id: NodeID = contact.get_rx_node();
            data[tx_node_id as usize].receivers[rx_node_id as usize]
                .contacts_to_receiver
                .push(Rc::new(RefCell::new(contact)));
        }

        Self {
            senders: data,
            nodes: all_refs,
            node_count,
        }
    }

    /// Applies exclusions to the nodes based on the provided sorted exclusions.
    ///
    /// Marks nodes as excluded if their index is in the `exclusions` list, otherwise unmarks them.
    ///
    /// # Parameters
    ///
    /// * `exclusions: &Vec<NodeID>` - A sorted list of node IDs to exclude.
    pub fn apply_exclusions_sorted(&mut self, exclusions: &Vec<NodeID>) {
        let mut exclusion_idx = 0;
        let exclusion_len = exclusions.len();

        for (node_id, sender) in self.senders.iter_mut().enumerate() {
            if exclusion_idx < exclusion_len && exclusions[exclusion_idx] as usize == node_id {
                sender.node.borrow_mut().info.excluded = true;
                exclusion_idx += 1;
            } else {
                sender.node.borrow_mut().info.excluded = false;
            }
        }
    }

    /// Retrieves the total number of nodes in the multigraph.
    ///
    /// # Returns
    ///
    /// * `usize` - The total number of nodes.
    pub fn get_node_count(&self) -> usize {
        self.node_count
    }
}
