use crate::bundle::Bundle;
use crate::contact::Contact;
use crate::contact_manager::{ContactManager, ContactManagerTxData};
use crate::errors::ASABRError;
use crate::multigraph::Multigraph;
use crate::node::Node;
use crate::node_manager::NodeManager;
use crate::route_stage::ViaHop;
use crate::route_stage::{RouteStage, SharedRouteStage};
use crate::types::{Date, NodeID};
use std::cell::RefCell;
use std::rc::Rc;

#[cfg(feature = "contact_work_area")]
pub mod contact_parenting;
pub mod hybrid_parenting;
#[cfg(feature = "contact_suppression")]
pub mod limiting_contact;
pub mod node_parenting;

/// Data structure that holds the results of a pathfinding operation.
///
/// This struct encapsulates information necessary for the outcome of a pathfinding algorithm,
/// including the associated bundle, excluded nodes, and organized route stages by destination.
///
/// # Type Parameters
///
/// * `CM` - A generic type that implements the `ContactManager` trait.
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct PathFindingOutput<NM: NodeManager, CM: ContactManager> {
    /// The `Bundle` for which the pathfinding is being performed.
    pub bundle: Bundle,
    /// The `source` RouteStage from which the pathfinding is being performed.
    pub source: SharedRouteStage<NM, CM>,
    /// A list of `NodeID`s representing nodes that should be excluded from the pathfinding.
    pub excluded_nodes_sorted: Vec<NodeID>,
    /// A vector that contains a `RouteStage`s for a specific destination node ID as the index.
    pub by_destination: Vec<Option<SharedRouteStage<NM, CM>>>,
}

pub type SharedPathFindingOutput<NM, CM> = Rc<RefCell<PathFindingOutput<NM, CM>>>;

impl<NM: NodeManager, CM: ContactManager> PathFindingOutput<NM, CM> {
    /// Creates a new `PathfindingOutput` instance, initializing the `by_destination` vector
    /// with empty vectors for each destination node and sorting the excluded nodes.
    ///
    /// # Parameters
    ///
    /// * `bundle` - A reference to the `Bundle` that is part of the pathfinding operation.
    /// * `source` - The source RouteStage from which the pathfinding is being performed.
    /// * `excluded_nodes_sorted` - A vector of `NodeID`s representing nodes to be excluded.
    /// * `node_count` - The total number of nodes in the graph.
    ///
    /// # Returns
    ///
    /// A new `PathfindingOutput` instance.
    pub fn new(
        bundle: &Bundle,
        source: SharedRouteStage<NM, CM>,
        excluded_nodes_sorted: &[NodeID],
        node_count: usize,
    ) -> Self {
        let exclusions = excluded_nodes_sorted.to_vec();
        Self {
            bundle: bundle.clone(),
            source,
            excluded_nodes_sorted: exclusions,
            by_destination: vec![None; node_count],
        }
    }

    pub fn get_source_route(&self) -> SharedRouteStage<NM, CM> {
        self.source.clone()
    }

    /// Initializes the route for a given destination in the routing stage.
    ///
    /// Dijkstra finds the reverse path, this method set up the path.
    ///
    /// # Parameters
    ///
    /// * `destination` - The target node ID for the routing.
    pub fn init_for_destination(&self, destination: NodeID) -> Result<(), ASABRError> {
        if let Some(route) = self.by_destination[destination as usize].clone() {
            RouteStage::init_route(route)?;
        }
        Ok(())
    }
}

/// The `Pathfinding` trait provides the interface for implementing a pathfinding algorithm.
/// It requires methods for creating a new instance and determining the next hop in a route.
///
/// # Type Parameters
///
/// * `NM` - A generic type that implements the `NodeManager` trait.
/// * `CM` - A generic type that implements the `ContactManager` trait.
pub trait Pathfinding<NM: NodeManager, CM: ContactManager> {
    /// Creates a new instance of the pathfinding algorithm with the provided nodes and contacts.
    ///
    /// # Parameters
    ///
    /// * `nodes` - A vector of `Node`s that represents the graph nodes.
    /// * `contacts` - A vector of `Contact`s that represents the edges between nodes.
    ///
    /// # Returns
    ///
    /// A new instance of the struct implementing `Pathfinding`.
    fn new(multigraph: Rc<RefCell<Multigraph<NM, CM>>>) -> Self;

    /// Determines the next hop in the route for the given bundle, excluding specified nodes.
    ///
    /// # Parameters
    ///
    /// * `current_time` - The current time for the pathfinding operation.
    /// * `source` - The `NodeID` of the source node.
    /// * `bundle` - A reference to the `Bundle` being routed.
    /// * `excluded_nodes_sorted` - A vector of `NodeID`s that should be excluded from the pathfinding.
    ///
    /// # Returns
    ///
    /// A `PathfindingOutput` containing the results of the pathfinding operation.
    fn get_next(
        &mut self,
        current_time: Date,
        source: NodeID,
        bundle: &Bundle,
        excluded_nodes_sorted: &[NodeID],
    ) -> Result<PathFindingOutput<NM, CM>, ASABRError>;

    /// Get a shared pointer to the multigraph.
    ///
    /// # Returns
    ///
    /// * A shared pointer to the multigraph.
    fn get_multigraph(&self) -> Rc<RefCell<Multigraph<NM, CM>>>;
}

/// Attempts to make a hop (i.e., a transmission between nodes) for the given route stage and bundle,
/// checking potential contacts to determine the best hop.
///
/// # Parameters
///
/// * `first_contact_index` - The index of the first contact to consider (lazy pruning).
/// * `sndr_route` - A reference-counted, mutable `RouteStage` that represents the sender's current route.
/// * `bundle` - A reference to the `Bundle` that is being routed.
/// * `contacts` - A vector of reference-counted, mutable `Contact`s representing available transmission opportunities.
/// * `tx_node` - A reference-counted, mutable `Node` representing the transmitting node.
/// * `rx_node` - A reference-counted, mutable `Node` representing the receiving node.
///
/// # Returns
///
/// An `Option` containing a `RouteStage` if a suitable hop is found, or `None` if no valid hop is available.
fn try_make_hop<NM: NodeManager, CM: ContactManager>(
    first_contact_index: usize,
    sndr_route: &SharedRouteStage<NM, CM>,
    _bundle: &Bundle,
    contacts: &[Rc<RefCell<Contact<NM, CM>>>],
    tx_node: &Rc<RefCell<Node<NM>>>,
    rx_node: &Rc<RefCell<Node<NM>>>,
) -> Option<RouteStage<NM, CM>> {
    let mut index = 0;
    let mut final_data = ContactManagerTxData {
        tx_start: 0.0,
        tx_end: 0.0,
        delay: 0.0,
        expiration: 0.0,
        arrival: Date::MAX,
    };

    // If bundle processing is enabled, a mutable bundle copy is required to be attached to the RouteStage.
    #[cfg(feature = "node_proc")]
    let mut bundle_to_consider = sndr_route.borrow().bundle.clone();
    #[cfg(not(feature = "node_proc"))]
    let bundle_to_consider = _bundle;

    let sndr_route_borrowed = sndr_route.borrow();

    for (idx, contact) in contacts.iter().enumerate().skip(first_contact_index) {
        let contact_borrowed = contact.borrow();

        #[cfg(feature = "contact_suppression")]
        if contact_borrowed.suppressed {
            continue;
        }

        if contact_borrowed.info.start > final_data.arrival {
            break;
        }

        #[cfg(feature = "node_proc")]
        let sending_time = tx_node
            .borrow()
            .manager
            .dry_run_process(sndr_route_borrowed.at_time, &mut bundle_to_consider);
        #[cfg(not(feature = "node_proc"))]
        let sending_time = sndr_route_borrowed.at_time;

        if let Some(hop) = contact_borrowed.manager.dry_run_tx(
            &contact_borrowed.info,
            sending_time,
            &bundle_to_consider,
        ) {
            #[cfg(feature = "node_tx")]
            if !tx_node.borrow().manager.dry_run_tx(
                sending_time,
                hop.tx_start,
                hop.tx_end,
                &bundle_to_consider,
            ) {
                continue;
            }

            if hop.tx_end + hop.delay < final_data.arrival {
                #[cfg(feature = "node_rx")]
                if !rx_node.borrow().manager.dry_run_rx(
                    hop.tx_start + hop.delay,
                    hop.tx_end + hop.delay,
                    _bundle,
                ) {
                    continue;
                }

                final_data = hop;
                index = idx;
            }
        }
    }

    if final_data.arrival < Date::MAX {
        let seleted_contact = &contacts[index];
        let mut route_proposition: RouteStage<NM, CM> = RouteStage::new(
            final_data.arrival,
            seleted_contact.borrow().get_rx_node(),
            Some(ViaHop {
                contact: seleted_contact.clone(),
                parent_route: sndr_route.clone(),
                tx_node: tx_node.clone(),
                rx_node: rx_node.clone(),
            }),
            #[cfg(feature = "node_proc")]
            bundle_to_consider,
        );

        route_proposition.hop_count = sndr_route_borrowed.hop_count + 1;
        route_proposition.cumulative_delay =
            sndr_route_borrowed.cumulative_delay + final_data.delay;
        route_proposition.expiration = Date::min(
            final_data.expiration - sndr_route_borrowed.cumulative_delay,
            sndr_route_borrowed.expiration,
        );

        return Some(route_proposition);
    }
    None
}
