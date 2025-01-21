use crate::contact::Contact;
use crate::contact_manager::{ContactManager, TxEndHopData};
use crate::multigraph::Multigraph;
use crate::node::Node;
use crate::node_manager::NodeManager;
use crate::route_stage::ViaHop;
use crate::types::{Date, NodeID};
use crate::{bundle::Bundle, route_stage::RouteStage};
use std::cell::RefCell;
use std::rc::Rc;

#[cfg(feature = "contact_work_area")]
pub mod contact_graph;
#[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
pub mod first_depleted;
#[cfg(feature = "contact_suppression")]
pub mod first_ending;
pub mod mpt;
pub mod node_graph;

/// Data structure that holds the results of a pathfinding operation.
///
/// This struct encapsulates information necessary for the outcome of a pathfinding algorithm,
/// including the associated bundle, excluded nodes, and organized route stages by destination.
///
/// # Type Parameters
///
/// * `CM` - A generic type that implements the `ContactManager` trait.
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct PathFindingOutput<CM: ContactManager> {
    /// The `Bundle` for which the pathfinding is being performed.
    pub bundle: Bundle,
    /// The `source` RouteStage from which the pathfinding is being performed.
    pub source: Rc<RefCell<RouteStage<CM>>>,
    /// A list of `NodeID`s representing nodes that should be excluded from the pathfinding.
    pub excluded_nodes_sorted: Vec<NodeID>,
    /// A vector that contains a `RouteStage`s for a specific destination node ID as the index.
    pub by_destination: Vec<Option<Rc<RefCell<RouteStage<CM>>>>>,
}

impl<CM: ContactManager> PathFindingOutput<CM> {
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
        source: Rc<RefCell<RouteStage<CM>>>,
        excluded_nodes_sorted: &Vec<NodeID>,
        node_count: usize,
    ) -> Self {
        let exclusions = excluded_nodes_sorted.clone();
        Self {
            bundle: bundle.clone(),
            source,
            excluded_nodes_sorted: exclusions,
            by_destination: vec![None; node_count],
        }
    }

    pub fn get_source_route(&self) -> Rc<RefCell<RouteStage<CM>>> {
        return self.source.clone();
    }

    /// Initializes the route for a given destination in the routing stage.
    ///
    /// Dijkstra finds the reverse path, this method set up the path.
    ///
    /// # Parameters
    ///
    /// * `destination` - The target node ID for the routing.
    pub fn init_for_destination(&self, destination: NodeID) {
        if let Some(route) = self.by_destination[destination as usize].clone() {
            RouteStage::init_route(route);
        }
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
        excluded_nodes_sorted: &Vec<NodeID>,
    ) -> PathFindingOutput<CM>;

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
    sndr_route: &Rc<RefCell<RouteStage<CM>>>,
    bundle: &Bundle,
    contacts: &Vec<Rc<RefCell<Contact<CM>>>>,
    tx_node: &Rc<RefCell<Node<NM>>>,
    rx_node: &Rc<RefCell<Node<NM>>>,
) -> Option<RouteStage<CM>> {
    let mut index = 0;
    let mut final_data = TxEndHopData {
        tx_start: 0.0,
        tx_end: 0.0,
        delay: 0.0,
        expiration: 0.0,
        arrival: Date::MAX,
    };

    // If bundle processing is enabled, a mutable bundle copy is required to be attached to the RouteStage.
    #[cfg(feature = "bundle_processing")]
    let mut bundle_to_consider = sndr_route.borrow().bundle_opt.clone();
    #[cfg(not(feature = "bundle_processing"))]
    let bundle_to_consider = bundle;

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

        #[cfg(feature = "bundle_processing")]
        let sending_time = tx_node
            .borrow()
            .manager
            .dry_run_process(sndr_route_borrowed.at_time, &mut bundle_to_consider);
        #[cfg(not(feature = "bundle_processing"))]
        let sending_time = sndr_route_borrowed.at_time;

        if let Some(hop) = contact_borrowed.manager.dry_run(
            &contact_borrowed.info,
            sending_time,
            &bundle_to_consider,
        ) {
            #[cfg(feature = "enable_node_management")]
            if !tx_node
                .borrow()
                .manager
                .dry_run_tx(hop.tx_start, hop.tx_end, &bundle_to_consider)
            {
                continue;
            }

            if hop.tx_end + hop.delay < final_data.arrival {
                #[cfg(feature = "enable_node_management")]
                if !rx_node.borrow().manager.dry_run_rx(
                    hop.tx_start + hop.delay,
                    hop.tx_end + hop.delay,
                    bundle,
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
        let mut route_proposition: RouteStage<CM> = RouteStage::new(
            final_data.arrival,
            seleted_contact.borrow().get_rx_node(),
            Some(ViaHop {
                contact: seleted_contact.clone(),
                parent_route: sndr_route.clone(),
            }),
            #[cfg(feature = "bundle_processing")]
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

/// Retrieves the next `Contact` to suppress based on the provided suppression function.
///
/// This function navigates through the provided route stage to identify the `Contact` that
/// is best suited for suppression, according to the specified comparison function
/// (`better_for_suppression_than_fn`). It iterates through the route's contacts to determine
/// the one that should be suppressed next.
///
/// # Parameters
///
/// * `route` - A reference-counted, mutable `RouteStage` representing the current routing stage.
/// * `better_for_suppression_than_fn` - A function pointer used to compare two `Contact`s and
///   determine which is better for suppression.
///
/// # Returns
///
/// An `Option` containing a reference-counted, mutable `Contact` that should be suppressed, if one
/// is found; otherwise, `None`.
#[cfg(feature = "contact_suppression")]
pub fn get_next_to_suppress<CM: ContactManager>(
    route: Rc<RefCell<RouteStage<CM>>>,
    better_for_suppression_than_fn: fn(&Contact<CM>, &Contact<CM>) -> bool,
) -> Option<Rc<RefCell<Contact<CM>>>> {
    let mut to_suppress_opt: Option<Rc<RefCell<Contact<CM>>>> = None;
    let mut next_route_option = Some(route);
    while let Some(curr_route) = next_route_option.take() {
        {
            let route_borrowed = curr_route.borrow();
            if let Some(ref via) = route_borrowed.via {
                match to_suppress_opt {
                    Some(ref to_suppress) => {
                        if better_for_suppression_than_fn(
                            &via.contact.borrow(),
                            &to_suppress.borrow(),
                        ) {
                            to_suppress_opt = Some(via.contact.clone());
                        }
                    }
                    None => to_suppress_opt = Some(via.contact.clone()),
                }
                next_route_option = Some(Rc::clone(&via.parent_route));
            }
        }
    }
    to_suppress_opt
}

/// Creates a new variant of the alternative pathfinding algorithm with a custom suppression strategy.
///
/// This macro generates a new struct that implements the `Pathfinding` trait, adding the ability to
/// suppress specific contacts during the routing process. The suppression logic is determined by the
/// provided `better_fn`, which compares `Contact`s to decide which should be suppressed.
///
/// # Parameters
///
/// * `$struct_name` - The name of the struct to be created.
/// * `$better_fn` - The name of the function used to compare two `Contact`s and determine which one
///   is better for suppression.
///
/// # Generated Struct
///
/// The generated struct will contain the following fields:
/// * `pathfinding` - An instance of the underlying pathfinding algorithm.
/// * `next_to_suppress` - An optional `Contact` that will be suppressed before the pathfinding stage.
///
/// The struct implements the `Pathfinding` trait, using the specified suppression strategy to
/// modify its behavior when selecting the next route. The `next_to_suppress` contact is removed before
/// tree construction, and prepare the new `next_to_suppress` contact.
#[cfg(feature = "contact_suppression")]
#[macro_export]
macro_rules! create_new_alternative_path_variant {
    ($struct_name:ident, $better_fn:ident) => {
        /// An alternative path finding algorithm (macro generated).
        ///
        /// Each time a new route must generated, a contact of the last found route is suppressed.
        #[doc = concat!("`", stringify!($struct_name), "` uses the `", stringify!($better_fn), "` function to select the next contact to suppress.")]
        /// This is macro generated check the documentation of `create_new_alternative_path_variant` for details.
        ///
        /// # Type Parameters
        ///
        /// * `NM` - A type that implements the `NodeManager` trait.
        /// * `CM` - A type that implements the `ContactManager` trait.
        /// * `D` - A type that implements the `Distance<CM>` trait.
        /// * `P` - A type that implements the `Pathfinding<NM, CM>` trait.
        pub struct $struct_name<
            NM: crate::node_manager::NodeManager,
            CM: ContactManager,
            P: crate::pathfinding::Pathfinding<NM, CM>,
        > {
            /// The underlying pathfinding algorithm used to find individual paths.
            pathfinding: P,
            suppression_map: Vec<Vec<std::rc::Rc<std::cell::RefCell<Contact<CM>>>>>,

            #[doc(hidden)]
            _phantom_nm: std::marker::PhantomData<NM>,
            #[doc(hidden)]
            _phantom_cm: std::marker::PhantomData<CM>,
        }

        impl<
                NM: crate::node_manager::NodeManager,
                CM: ContactManager,
                P: crate::pathfinding::Pathfinding<NM, CM>,
            > crate::pathfinding::Pathfinding<NM, CM> for $struct_name<NM, CM, P>
        {
            #[doc = concat!("Constructs a new `", stringify!($struct_name), "` instance with the provided nodes and contacts.")]
            ///
            /// Generated with a macro, check the macro documentation for details.
            ///
            /// # Parameters
            ///
            /// * `multigraph` - A shared pointer to a multigraph.
            ///
            /// # Returns
            ///
            #[doc = concat!("* `Self` - A new instance of `", stringify!($struct_name), "`.")]
            fn new(
                multigraph: std::rc::Rc<std::cell::RefCell<crate::multigraph::Multigraph<NM, CM>>>
            ) -> Self {
                let node_count = multigraph.borrow().get_node_count();
                Self {

                    pathfinding: P::new(multigraph),
                    suppression_map: vec![Vec::new(); node_count],
                    _phantom_nm: std::marker::PhantomData,
                    _phantom_cm: std::marker::PhantomData,
                }
            }
            /// Finds the next route based on the current state and available contacts.
            ///
            /// # Parameters
            ///
            /// * `current_time` - The current time used for evaluating routes.
            /// * `source` - The `NodeID` of the source node from which to begin pathfinding.
            /// * `bundle` - The `Bundle` associated with the pathfinding operation.
            /// * `excluded_nodes_sorted` - A list of `NodeID`s to be excluded from the pathfinding.
            ///
            /// # Returns
            ///
            /// * `PathfindingOutput<CM>` - The resulting pathfinding output, including the routes found.
            fn get_next(
                &mut self,
                current_time: crate::types::Date,
                source: crate::types::NodeID,
                bundle: &crate::bundle::Bundle,
                excluded_nodes_sorted: &Vec<crate::types::NodeID>,
            ) -> super::PathFindingOutput<CM> {

                self.suppression_map[bundle.destinations[0] as usize].retain(|contact| {
                    if contact.borrow().info.end < current_time {
                        false
                    } else {
                        contact.borrow_mut().suppressed = true;
                        true
                    }
                });

                let tree = self
                    .pathfinding
                    .get_next(current_time, source, bundle, excluded_nodes_sorted);

                if let Some(route) = tree.by_destination[bundle.destinations[0] as usize].clone() {
                    if let Some(contact) = crate::pathfinding::get_next_to_suppress(route, $better_fn) {
                        self.suppression_map[bundle.destinations[0] as usize].push(contact);
                    }
                }
                for contact in &self.suppression_map[bundle.destinations[0] as usize] {
                    contact.borrow_mut().suppressed = false;
                }

                return tree;
            }

            /// Get a shared pointer to the multigraph.
            ///
            /// # Returns
            ///
            /// * A shared pointer to the multigraph.
            fn get_multigraph(&self) -> std::rc::Rc<std::cell::RefCell<crate::multigraph::Multigraph<NM, CM>>> {
                return self.pathfinding.get_multigraph();
            }
        }
    };
}
