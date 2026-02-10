use std::cell::RefCell;
use std::rc::Rc;

use super::node::Node;
use crate::contact::Contact;
use crate::contact_manager::ContactManager;
use crate::errors::ASABRError;
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
    pub contacts_to_receiver: Vec<Rc<RefCell<Contact<NM, CM>>>>,
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
        None
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
    /// Note: For Dijkstra, we need fast access for the senders. To this end, the index
    /// in the "senders" Vec matches the  transmitter NodeID. There is a small memory
    /// overhead if some nodes are not transmitters in the contact plan. Regarding the
    /// receivers, only fast iteration is required. The indices of the senders[tx_id].receivers
    /// Vec do not match the receivers NodeID, and no entry exists if a node never receives.
    ///
    /// # Parameters
    ///
    /// * `nodes` - A vector of nodes to be included in the multigraph.
    /// * `contact_plan` - A vector of contacts that define the connections between nodes.
    ///
    /// # Returns
    ///
    /// * `Self` - A new instance of `Multigraph`.
    pub fn new(mut nodes: Vec<Node<NM>>, mut contact_plan: Vec<Contact<NM, CM>>) -> Self {
        // the contact plan might not be sorted
        // having a sorted list of contacts allow easy multigraph creation
        let node_count = nodes.len();
        let mut senders: Vec<Sender<NM, CM>> = Vec::with_capacity(node_count);

        contact_plan.sort_unstable();
        nodes.sort_unstable();

        let mut all_refs = Vec::with_capacity(node_count);

        for node in nodes {
            let node_ref = Rc::new(RefCell::new(node));
            // to avoid realloc and preprocessing to get the perfect layout
            // we just alloc with the worst case capacity and we shrink later
            senders.push(Sender {
                node: Rc::clone(&node_ref),
                receivers: Vec::with_capacity(node_count),
            });
            all_refs.push(node_ref)
        }

        while let Some(last_contact) = contact_plan.last() {
            let tx_id = last_contact.get_tx_node();
            let rx_id = last_contact.get_rx_node();

            let mut contact_count_to_drain = 0;

            for contact in contact_plan.iter().rev() {
                if contact.get_rx_node() != rx_id as NodeID
                    || contact.get_tx_node() != tx_id as NodeID
                {
                    break;
                }
                contact_count_to_drain += 1;
            }

            let first_to_drain = contact_plan.len() - contact_count_to_drain;
            let mut contacts_to_receiver = Vec::with_capacity(contact_count_to_drain);
            let drain = contact_plan.drain(first_to_drain..);

            for contact in drain {
                contacts_to_receiver.push(Rc::new(RefCell::new(contact)));
            }

            senders[tx_id as usize].receivers.push(Receiver {
                node: all_refs[rx_id as usize].clone(),
                contacts_to_receiver,
                next: 0,
            });
        }

        for sender in &mut senders {
            sender.receivers.shrink_to_fit();
        }

        Self {
            senders,
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
    /// * `exclusions: &[NodeID]` - A sorted list of node IDs to exclude.
    pub fn prepare_for_exclusions_sorted(
        &mut self,
        exclusions: &[NodeID],
    ) -> Result<(), ASABRError> {
        let mut exclusion_idx = 0;
        let exclusion_len = exclusions.len();

        for (node_id, sender) in self.senders.iter_mut().enumerate() {
            if exclusion_idx < exclusion_len && exclusions[exclusion_idx] as usize == node_id {
                sender.node.try_borrow_mut()?.info.excluded = true;
                exclusion_idx += 1;
            } else {
                sender.node.try_borrow_mut()?.info.excluded = false;
            }
        }
        Ok(())
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
