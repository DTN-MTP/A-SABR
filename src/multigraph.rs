use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::node::Node;
use crate::contact::Contact;
use crate::contact_manager::ContactManager;
use crate::contact_plan::ContactPlan;
use crate::errors::ASABRError;
use crate::node_manager::NodeManager;
use crate::types::*;
use crate::vertex::{Vertex, VertexID};

/// Represents a sender node in a routing system, with associated receivers.
///
/// The `Sender` struct holds the ID of a sender vertex and a list of `Receiver`
/// instances that represent the intended recipients for messages or routing actions.
///
/// # Generic Parameters
/// - `NM`: A type implementing the `NodeManager` trait, responsible for managing node-level operations.
/// - `CM`: A type implementing the `ContactManager` trait, responsible for managing contact-level operations.
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Sender<NM: NodeManager, CM: ContactManager> {
    /// The ID of the vertex represented by this sender.
    pub vertex_id: VertexID,
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
    /// The ID of the vertex represented by this receiver.
    pub vertex_id: VertexID,
    /// A list of contacts providing paths to this receiver.
    pub contacts_to_receiver: Vec<Rc<RefCell<Contact<NM, CM>>>>,
    /// The index of the next contact to be checked for relevance.
    pub next: RefCell<usize>,
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
    pub fn lazy_prune_and_get_first_idx(&self, current_time: Date) -> Option<usize> {
        let mut next_mut = self.next.borrow_mut();
        for (idx, contact) in self.contacts_to_receiver.iter().enumerate().skip(*next_mut) {
            if contact.borrow().info.end > current_time {
                *next_mut = idx;
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
    pub fn is_excluded(&self, nodes: &[Rc<RefCell<Node<NM>>>]) -> bool {
        if self.vertex_id as usize >= nodes.len() {
            // It's a vnode
            return false;
        }
        return nodes[self.vertex_id as usize].borrow().info.excluded;
    }
}

/// Represents a multigraph structure, where each node can have multiple connections.
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Multigraph<NM: NodeManager, CM: ContactManager> {
    /// The list of sender objects.
    pub senders: Vec<Sender<NM, CM>>,
    /// The list of node objects.
    pub real_nodes: Vec<Rc<RefCell<Node<NM>>>>,
    /// The total number of nodes in the multigraph.
    vertex_count: usize,
}

impl<NM: NodeManager, CM: ContactManager> Multigraph<NM, CM> {
    /// Creates a new `Multigraph` from a contact plan.
    ///
    /// Note: For Dijkstra, we need fast access for the senders. To this end, the index
    /// in the "senders" Vec matches the  transmitter NodeID. There is a small memory
    /// overhead if some nodes are not transmitters in the contacts. Regarding the
    /// receivers, only fast iteration is required. The indices of the senders[tx_id].receivers
    /// Vec do not match the receivers NodeID, and no entry exists if a node never receives.
    ///
    /// # Parameters
    ///
    /// * `ContactPlan` - A contact plan of nodes, contacts and a vnode map, and associated management information.
    ///
    /// # Returns
    ///
    /// * `Self` - A new instance of `Multigraph`.
    pub fn new(contact_plan: ContactPlan<NM, CM>) -> Result<Self, ASABRError> {
        // work area
        let vertex_count = contact_plan.vertices.len();
        let virtual_node_count = contact_plan.vnode_map.get_vnode_to_rids_map().len();
        let real_node_count = vertex_count - virtual_node_count;
        #[allow(clippy::type_complexity)]
        // Maps contacts to Sender vertex IDs and Receiver vertex IDs.
        let mut snd_rcv_map: HashMap<
            VertexID,
            HashMap<VertexID, Vec<Rc<RefCell<Contact<NM, CM>>>>>,
        > = HashMap::with_capacity(vertex_count);
        let mut is_interior = vec![false; real_node_count];

        // output
        let mut nodes = Vec::with_capacity(real_node_count);
        let mut senders = Vec::with_capacity(vertex_count);

        // collect real nodes, track enodes, init senders
        for ver in contact_plan.vertices {
            if let Vertex::INode(node) = &ver {
                is_interior[node.get_node_id() as usize] = true;
            }

            match ver {
                Vertex::ENode(node) | Vertex::INode(node) => {
                    let id = node.get_node_id();
                    nodes.push(Rc::new(RefCell::new(node)));
                    senders.push(Sender {
                        vertex_id: id,
                        receivers: Vec::with_capacity(vertex_count),
                    });
                }
                Vertex::VNode(vid) => {
                    senders.push(Sender {
                        vertex_id: vid,
                        receivers: Vec::with_capacity(vertex_count),
                    });
                }
            }
        }

        let vnodes_for_rid = contact_plan.vnode_map.get_rid_to_vnodes_map();

        // Fill contacts into vertex Sender and Receiver pairs (including vnodes) in the map.
        for contact in contact_plan.contacts {
            let real_tx_id = contact.get_tx_node_id();
            let real_rx_id = contact.get_rx_node_id();

            let contact_rc = Rc::new(RefCell::new(contact));

            for t in vnodes_for_rid
                .get(&real_tx_id)
                .into_iter()
                .flatten()
                .chain(std::iter::once(&real_tx_id))
            {
                for r in vnodes_for_rid
                    .get(&real_rx_id)
                    .into_iter()
                    .flatten()
                    .chain(std::iter::once(&real_rx_id))
                {
                    snd_rcv_map
                        .entry(*t)
                        .or_default()
                        .entry(*r)
                        .or_default()
                        .push(contact_rc.clone());
                }
            }
        }

        // now "flatten" (move to linear data structure) and shrink receivers
        for (t, receivers) in snd_rcv_map {
            for (r, mut contacts) in receivers {
                if (t as usize) < real_node_count && (r as usize) < real_node_count {
                    contacts.sort_unstable();
                } else {
                    // A vnode Sender or Receiver's contacts must be sorted by time only, not by
                    // Tx/Rx node ID.
                    contacts.sort_unstable_by(|a, b| a.borrow().cmp_by_start(&b.borrow()))
                }
                let recver = Receiver {
                    vertex_id: r,
                    contacts_to_receiver: contacts,
                    next: 0.into(),
                };
                senders[t as usize].receivers.push(recver);
            }
            senders[t as usize].receivers.shrink_to_fit();
        }

        Ok(Self {
            senders,
            real_nodes: nodes,
            vertex_count,
        })
    }

    /// Applies exclusions to the nodes based on the provided sorted exclusions.
    ///
    /// Marks nodes as excluded if their index is in the `exclusions` list, otherwise unmarks them.
    ///
    /// # Parameters
    ///
    /// * `exclusions: &[NodeID]` - A sorted list of node IDs to exclude.
    ///
    /// # Returns
    /// - `Ok(())`: If all exclusions were applied successfully.
    /// - Err(ASABRError)`: If a node cannot be mutably borrowed.
    pub fn prepare_for_exclusions_sorted(
        &mut self,
        exclusions: &[NodeID],
    ) -> Result<(), ASABRError> {
        let mut exclusion_idx = 0;
        let exclusion_len = exclusions.len();

        for (node_id, node) in self.real_nodes.iter_mut().enumerate() {
            if exclusion_idx < exclusion_len && exclusions[exclusion_idx] as usize == node_id {
                node.try_borrow_mut()?.info.excluded = true;
                exclusion_idx += 1;
            } else {
                node.try_borrow_mut()?.info.excluded = false;
            }
        }
        Ok(())
    }

    /// Retrieves the total number of vertices in the multigraph.
    ///
    /// # Returns
    ///
    /// * `usize` - The total number of vertices.
    pub fn get_vertex_count(&self) -> usize {
        self.vertex_count
    }
}
