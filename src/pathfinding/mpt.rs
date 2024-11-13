use std::{cell::RefCell, cmp::Reverse, collections::BinaryHeap, rc::Rc};

use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    distance::Distance,
    multigraph::Multigraph,
    node_manager::NodeManager,
    route_stage::RouteStage,
    types::{Date, NodeID},
};

/// A structure representing a work area for multi-path tracking (MPT) pathfinding.
///
/// `MptWorkArea` maintains information about the current routing state, including
/// the initial bundle, the source route stage, excluded nodes, and routes grouped by destination.
/// This structure is used in pathfinding operations to manage and organize route stages for
/// efficient routing in a multi-destination network.
///
/// This type is designed to derive easily a PathFindingOutput from this work area.
///
/// # Type Parameters
/// - `CM`: A type implementing the `ContactManager` trait, which handles contacts for routing.
/// - `D`: A type implementing the `Distance<CM>` trait, defining the distance metric for route comparison.
struct MptWorkArea<CM: ContactManager, D: Distance<CM>> {
    /// The bundle associated with this work area.
    pub bundle: Bundle,
    /// The source route stage, representing the starting point for routing.
    pub source: Rc<RefCell<RouteStage<CM, D>>>,
    /// A sorted list of node IDs to be excluded from routing paths.
    pub excluded_nodes_sorted: Vec<NodeID>,
    /// A vector containing vectors of route stages, grouped by destination.
    /// Each inner vector represents possible routes to a specific destination,
    /// sorted in order of preference.
    pub by_destination: Vec<Vec<Rc<RefCell<RouteStage<CM, D>>>>>,
}

impl<CM: ContactManager, D: Distance<CM>> MptWorkArea<CM, D> {
    /// Creates a new `MptWorkArea` instance, initializing it with the given bundle,
    /// source route, excluded nodes, and a specified number of destination nodes.
    ///
    /// # Parameters
    /// - `bundle`: A reference to the `Bundle` representing the data payload for routing.
    /// - `source`: An `Rc<RefCell<RouteStage<CM, D>>>` reference to the initial route stage.
    /// - `excluded_nodes_sorted`: A reference to a sorted vector of `NodeID`s to be excluded from routing paths.
    /// - `node_count`: The number of destination nodes, which determines the size of `by_destination`.
    ///
    /// # Returns
    /// A new instance of `MptWorkArea` initialized with the provided parameters.
    pub fn new(
        bundle: &Bundle,
        source: Rc<RefCell<RouteStage<CM, D>>>,
        excluded_nodes_sorted: &Vec<NodeID>,
        node_count: usize,
    ) -> Self {
        let exclusions = excluded_nodes_sorted.clone();
        Self {
            bundle: bundle.clone(),
            source,
            excluded_nodes_sorted: exclusions,
            by_destination: vec![Vec::new(); node_count],
        }
    }

    /// Converts this `MptWorkArea` into a `PathFindingOutput`, organizing routes for each destination.
    ///
    /// This function creates a `PathFindingOutput` by selecting the preferred route (if any) for each
    /// destination in `by_destination`. For each destination, if a route exists, it is added to the output;
    /// otherwise, `None` is added to indicate no viable route.
    ///
    /// # Returns
    /// A `PathFindingOutput<CM, D>` containing the bundle, source route stage, excluded nodes,
    /// and selected routes by destination.
    pub fn to_pathfinding_output(self) -> PathFindingOutput<CM, D> {
        let mut options = Vec::new();

        for routes in &self.by_destination {
            if routes.is_empty() {
                options.push(None);
            } else {
                options.push(Some(routes[0].clone()));
            }
        }

        return PathFindingOutput {
            bundle: self.bundle,
            source: self.source,
            excluded_nodes_sorted: self.excluded_nodes_sorted.clone(),
            by_destination: options,
        };
    }
}

use super::{try_make_hop, PathFindingOutput, Pathfinding};

/// Attempts to insert a new route proposal into the pathfinding output tree.
///
/// This function checks if the proposed route is strictly or partially better than existing
/// routes for the specified receiver node. If it is better, the function updates the routes
/// accordingly and disables less favorable routes.
///
/// # Parameters
///
/// * `proposition` - The `RouteStage` representing the new route proposal.
/// * `tree` - A mutable reference to the `PathfindingOutput` where the routes are stored.
///
/// # Returns
///
/// * `Option<Rc<RefCell<RouteStage<CM, D>>>>` - Returns an `Option` containing a reference to the
///   newly inserted route if the insertion was successful; otherwise, returns `None`.
fn try_insert<CM: ContactManager, D: Distance<CM>>(
    proposition: RouteStage<CM, D>,
    tree: &mut MptWorkArea<CM, D>,
) -> Option<Rc<RefCell<RouteStage<CM, D>>>> {
    let routes_for_rx_node = &mut tree.by_destination[proposition.to_node as usize];
    let mut insert_index: usize = 0;
    let mut insert = false;

    if routes_for_rx_node.is_empty() {
        let proposition_rc = Rc::new(RefCell::new(proposition));
        routes_for_rx_node.push(Rc::clone(&proposition_rc));
        return Some(proposition_rc);
    }

    while insert_index < routes_for_rx_node.len() {
        // Strictly better for:
        // - arrival time
        let route = routes_for_rx_node[insert_index].borrow();

        if proposition.at_time < route.at_time {
            insert = true;
            break;
        }
        if proposition.at_time == route.at_time {
            // - hop count
            if proposition.hop_count < route.hop_count {
                insert = true;
                break;
            }
            if proposition.hop_count == route.hop_count {
                // - expiration time
                if proposition.expiration > route.expiration {
                    insert = true;
                    break;
                }
            }
        // Partially better for :
        } else {
            // - maybe expiration : too risky
            if proposition.hop_count > route.hop_count {
                insert = false;
                break;
            }
            if proposition.hop_count == route.hop_count {
                // - nothing : reject
                if proposition.expiration <= route.expiration {
                    insert = false;
                    break;
                }
            } else {
                // - hop count : ok be verify a next know route is not better
                insert = true;
            }
        }
        insert_index += 1;
    }

    let mut truncate_index = insert_index + 1;
    if insert {
        // detect the first prune event but do nothing
        while truncate_index < routes_for_rx_node.len() {
            let route = &routes_for_rx_node[truncate_index].borrow();
            if proposition.at_time <= route.at_time && proposition.hop_count <= route.hop_count {
                break;
            }
            truncate_index += 1
        }

        // Now disable the routes(for the shared ref in the priority queue)
        for idx in (truncate_index)..routes_for_rx_node.len() {
            routes_for_rx_node[idx].borrow_mut().is_disabled = true;
        }

        // Now truncate
        routes_for_rx_node.truncate(truncate_index);

        let proposition_rc = Rc::new(RefCell::new(proposition));
        // if everything was truncated, the following has no overhead
        routes_for_rx_node.insert(insert_index, Rc::clone(&proposition_rc));
        return Some(proposition_rc);
    }

    None
}

macro_rules! define_mpt {
    ($name:ident, $is_tree_output:tt, $with_exclusions:tt) => {
        /// A multipath tracking (SPSN v2) implementation of Dijkstra algorithm.
        ///
        /// Use this implementation for optimized pahtfinding precision.
        ///
        /// # Type Parameters
        ///
        /// * `NM` - A type that implements the `NodeManager` trait.
        /// * `CM` - A type that implements the `ContactManager` trait.
        /// * `D` - A type that implements the `Distance<CM>` trait.
        pub struct $name<NM: NodeManager, CM: ContactManager, D: Distance<CM>> {
            /// The node multigraph for contact access.
            graph: Rc<RefCell<Multigraph<NM, CM, D>>>,
        }

        impl<NM: NodeManager, CM: ContactManager, D: Distance<CM>> Pathfinding<NM, CM, D>
            for $name<NM, CM, D>
        {
            /// Constructs a new `Mpt` instance with the provided nodes and contacts.
            ///
            /// # Parameters
            ///
            /// * `multigraph` - A shared pointer to a multigraph.
            ///
            /// # Returns
            ///
            #[doc = concat!( " * `Self` - A new instance of `",stringify!($name),"`.")]
            fn new(multigraph: Rc<RefCell<Multigraph<NM, CM, D>>>) -> Self {
                Self { graph: multigraph }
            }

            /// Finds the next route based on the current state and available contacts.
            ///
            /// This method uses a priority queue to explore potential routes from a source node,
            /// considering the current time, bundle, and excluded nodes.
            ///
            /// # Parameters
            ///
            /// * `current_time` - The current time used for evaluating routes.
            /// * `source` - The `NodeID` of the source node from which to begin pathfinding.
            /// * `bundle` - The `Bundle` associated with the pathfinding operation.
            /// * `excluded_nodes` - A list of `NodeID`s to be excluded from the pathfinding.
            ///
            /// # Returns
            ///
            /// * `PathfindingOutput<CM, D>` - The resulting pathfinding output, including the routes found.
            fn get_next(
                &mut self,
                current_time: Date,
                source: NodeID,
                bundle: &Bundle,
                excluded_nodes_sorted: &Vec<NodeID>,
            ) -> PathFindingOutput<CM, D> {
                let mut graph = self.graph.borrow_mut();
                if $with_exclusions {
                    graph.apply_exclusions_sorted(excluded_nodes_sorted);
                }
                let source_route: Rc<RefCell<RouteStage<CM, D>>> =
                    Rc::new(RefCell::new(RouteStage::new(current_time, source, None)));
                let mut tree: MptWorkArea<CM, D> = MptWorkArea::new(
                    bundle,
                    source_route.clone(),
                    excluded_nodes_sorted,
                    graph.get_node_count(),
                );
                let mut priority_queue = BinaryHeap::new();

                tree.by_destination[source as usize].push(source_route.clone());
                priority_queue.push(Reverse(Rc::clone(&source_route)));

                while let Some(Reverse(from_route)) = priority_queue.pop() {
                    if from_route.borrow().is_disabled {
                        continue;
                    }

                    let tx_node_id = from_route.borrow().to_node;

                    if !$is_tree_output {
                        if bundle.destinations[0] == tx_node_id {
                            break;
                        }
                    }

                    let sender = &mut graph.senders[tx_node_id as usize];

                    for receiver in &mut sender.receivers {
                        if $with_exclusions {
                            if receiver.is_excluded() {
                                continue;
                            }
                        }

                        if let Some(first_contact_index) =
                            receiver.lazy_prune_and_get_first_idx(current_time)
                        {
                            if let Some(route_proposition) = try_make_hop(
                                first_contact_index,
                                &from_route,
                                bundle,
                                &receiver.contacts_to_receiver,
                                &sender.node,
                                &receiver.node,
                            ) {
                                // This transforms a prop in the stack to a prop in the heap
                                if let Some(new_route) = try_insert(route_proposition, &mut tree) {
                                    priority_queue.push(Reverse(new_route.clone()));
                                }
                            }
                        }
                    }
                }

                // totally fine as we have Rcs
                for v in &mut tree.by_destination {
                    v.truncate(1);
                }

                return tree.to_pathfinding_output();
            }

            /// Get a shared pointer to the multigraph.
            ///
            /// # Returns
            ///
            /// * A shared pointer to the multigraph.
            fn get_multigraph(&self) -> Rc<RefCell<Multigraph<NM, CM, D>>> {
                return self.graph.clone();
            }
        }
    };
}

define_mpt!(MptTree, true, true);
define_mpt!(MptPath, false, false);
