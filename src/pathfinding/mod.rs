use crate::bundle::Bundle;
use crate::contact::Contact;
use crate::contact_manager::ContactManager;
use crate::errors::ASABRError;
use crate::multigraph::Multigraph;
use crate::node::Node;
use crate::node_manager::NodeManager;
use crate::route_stage::{eval_hop, HopMode, HopResult, RouteStage, SharedRouteStage, ViaHop};
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
/// * `NM` - A generic type that implements the `NodeManager` trait.
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
    /// Creates a new `PathFindingOutput` instance, initializing the `by_destination` vector
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
    /// A new `PathFindingOutput` instance.
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
    /// * `multigraph` - A reference-counted, mutable pointer to the multigraph containing nodes and contacts for pathfinding.
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
    /// A `Result<PathFindingOutput<NM, CM>, ASABRError>` containing the results of the pathfinding operation,
    /// or an error if the operation fails.
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
    let mut best_arrival = Date::MAX;
    let mut best_index = 0;
    let mut best_result: Option<HopResult> = None;

    let sndr_route_borrowed = sndr_route.borrow();

    // Base bundle: post-processing state stored in the route stage (node_proc), or the
    // original bundle otherwise.
    #[cfg(feature = "node_proc")]
    let base_bundle = &sndr_route_borrowed.bundle;
    #[cfg(not(feature = "node_proc"))]
    let base_bundle = _bundle;

    for (idx, contact) in contacts.iter().enumerate().skip(first_contact_index) {
        // Pre-flight checks on an immutable borrow before taking the mutable one.
        {
            let contact_b = contact.borrow();
            #[cfg(feature = "contact_suppression")]
            if contact_b.suppressed {
                continue;
            }
            if contact_b.info.start > best_arrival {
                break;
            }
        }

        let mut contact_b = contact.borrow_mut();
        let mut tx_b = tx_node.borrow_mut();
        let mut rx_b = rx_node.borrow_mut();

        if let Some(result) = eval_hop(
            &mut contact_b,
            &mut tx_b,
            &mut rx_b,
            sndr_route_borrowed.at_time,
            base_bundle,
            HopMode::DryRun,
        ) {
            if result.tx_data.arrival < best_arrival {
                best_arrival = result.tx_data.arrival;
                best_index = idx;
                best_result = Some(result);
            }
        }
    }

    if let Some(hop_result) = best_result {
        // Extract fields before any partial move of hop_result.
        let arrival = hop_result.tx_data.arrival;
        let delay = hop_result.tx_data.delay;
        let expiration = hop_result.tx_data.expiration;

        let selected_contact = &contacts[best_index];
        let mut route_proposition: RouteStage<NM, CM> = RouteStage::new(
            arrival,
            selected_contact.borrow().get_rx_node(),
            Some(ViaHop {
                contact: selected_contact.clone(),
                parent_route: sndr_route.clone(),
                tx_node: tx_node.clone(),
                rx_node: rx_node.clone(),
            }),
            #[cfg(feature = "node_proc")]
            hop_result.bundle,
        );

        route_proposition.hop_count = sndr_route_borrowed.hop_count + 1;
        route_proposition.cumulative_delay = sndr_route_borrowed.cumulative_delay + delay;
        route_proposition.expiration = Date::min(
            expiration - sndr_route_borrowed.cumulative_delay,
            sndr_route_borrowed.expiration,
        );

        return Some(route_proposition);
    }
    None
}
