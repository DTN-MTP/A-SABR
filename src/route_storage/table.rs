use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use crate::{
    bundle::Bundle, contact_manager::ContactManager, distance::Distance, node_manager::NodeManager,
    route_stage::RouteStage, routing::dry_run_unicast_path_with_exclusions, types::NodeID,
};

use super::{Route, RouteStorage};

/// A routing table that stores the routes for each destinations.
///
/// `RoutingTable` stores and selects the best available routes for bundles. The table allows
/// the storage of new routes and the selection of optimal routes based on the Distance<CM> trait.
///
/// # Type Parameters
/// - `NM`: A type implementing `NodeManager`, responsible for managing nodes.
/// - `CM`: A type implementing `ContactManager`, handling contacts within the network.
/// - `D`: A type implementing `Distance<CM>`, providing a distance metric for route comparison.
///
/// # Fields
/// - `tables`: A vector of vectors of `Route<CM, D>`, where each inner vector represents
///   routes to a specific destination node.
/// - `_phantom_nm`: A phantom marker to associate the routing table with a `NodeManager` type.
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct RoutingTable<NM: NodeManager, CM: ContactManager, D: Distance<CM>> {
    /// Routes are stored in a two-dimensional vector, grouped by destination node.
    tables: Vec<Vec<Route<CM, D>>>,
    #[doc(hidden)]
    _phantom_nm: PhantomData<NM>,
}

impl<NM: NodeManager, CM: ContactManager, D: Distance<CM>> RoutingTable<NM, CM, D> {
    /// Creates a new, empty `RoutingTable`.
    ///
    /// # Returns
    /// A new instance of `RoutingTable` with empty routes and initialized phantom type for
    /// `NodeManager`.
    pub fn new() -> Self {
        Self {
            tables: Vec::new(),
            // for compilation
            _phantom_nm: PhantomData,
        }
    }
}

impl<NM: NodeManager, CM: ContactManager, D: Distance<CM>> RouteStorage<NM, CM, D>
    for RoutingTable<NM, CM, D>
{
    /// Stores a new route for a given bundle in the routing table.
    ///
    /// This function associates the route with the destination of the bundle. If the
    /// destination index exceeds the current size of `tables`, the vector is resized to
    /// accommodate the new destination.
    ///
    /// # Parameters
    /// - `bundle`: The bundle whose destination will determine the storage index.
    /// - `route`: The `Route<CM, D>` to be stored.
    fn store(&mut self, bundle: &Bundle, route: Route<CM, D>) {
        let dest = bundle.destinations[0];
        if self.tables.len() < 1 + dest as usize {
            self.tables.resize((dest + 1) as usize, vec![])
        }
        self.tables[dest as usize].push(route);
    }

    /// Selects the best route for a bundle, based on current network conditions and
    /// the Distance<CM> trait.
    ///
    /// This function evaluates available routes to the bundle's destination, choosing the
    /// route that is most favorable according to the current time, node list. Routes are
    /// compared to find the best candidate, which is the returned.
    ///
    /// Apply the exclusions to the node objects before calling this function.
    ///
    /// # Parameters
    /// - `bundle`: The bundle for which a route is being selected.
    /// - `curr_time`: The current time, used in route evaluation.
    /// - `node_list`: A list of nodes, provided as `Rc<RefCell<Node<NM>>>`, used to assess
    ///   the feasibility of the route.
    /// - `_excluded_nodes_sorted`: A list of nodes to exclude from routing, although not used
    ///   explicitly in this function.
    ///
    /// # Returns
    /// - `Some(Route<CM, D>)` if a suitable route is found.
    /// - `None` if no feasible route is available.
    fn select(
        &mut self,
        bundle: &Bundle,
        curr_time: crate::types::Date,
        node_list: &Vec<Rc<RefCell<crate::node::Node<NM>>>>,
        _excluded_nodes_sorted: &Vec<NodeID>,
    ) -> Option<Route<CM, D>> {
        let dest = bundle.destinations[0];

        if self.tables.len() < 1 + dest as usize {
            self.tables.resize((dest + 1) as usize, vec![])
        }

        let routes = &self.tables[dest as usize];

        let mut best_route_option: Option<Rc<RefCell<RouteStage<CM, D>>>> = None;
        let mut best_index = 0;

        for (index, route) in routes.iter().enumerate() {
            if let Some(new_candidate) = dry_run_unicast_path_with_exclusions(
                bundle,
                curr_time,
                route.source_stage.clone(),
                route.destination_stage.clone(),
                node_list,
            ) {
                match best_route_option {
                    Some(ref best_route) => {
                        if new_candidate < *best_route {
                            best_route_option = Some(new_candidate);
                            best_index = index;
                        }
                    }
                    None => best_route_option = Some(new_candidate),
                }
            }
        }

        if best_route_option != None {
            return Some(routes[best_index].clone());
        }
        None
    }
}
