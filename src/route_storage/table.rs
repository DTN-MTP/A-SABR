use std::{cell::RefCell, cmp::Ordering, marker::PhantomData, rc::Rc};

use crate::{
    bundle::Bundle, contact_manager::ContactManager, distance::Distance, errors::ASABRError,
    multigraph::Multigraph, node_manager::NodeManager, routing::dry_run_unicast_path,
    types::NodeID,
};

use super::{Route, RouteStorage};

/// A routing table that stores the routes for each destinations.
///
/// `RoutingTable` stores and selects the best available routes for bundles. The table allows
/// the storage of new routes and the selection of optimal routes based on the `Distance<NM, CM>` trait.
///
/// # Type Parameters
/// - `NM`: A type implementing `NodeManager`, responsible for managing nodes.
/// - `CM`: A type implementing `ContactManager`, handling contacts within the network.
/// - `D`: A type implementing `Distance<NM, CM>`, providing a distance metric for route comparison.
///
/// # Fields
/// - `tables`: A vector of vectors of `Route<NM, CM>`, where each inner vector represents
///   routes to a specific destination node.
/// - `_phantom_nm`: A phantom marker to associate the routing table with a `NodeManager` type.
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct RoutingTable<NM: NodeManager, CM: ContactManager, D: Distance<NM, CM>> {
    /// Routes are stored in a two-dimensional vector, grouped by destination node.
    tables: Vec<Vec<Route<NM, CM>>>,
    #[doc(hidden)]
    _phantom_nm: PhantomData<NM>,
    #[doc(hidden)]
    _phantom_distance: PhantomData<D>,
}

impl<NM: NodeManager, CM: ContactManager, D: Distance<NM, CM>> Default for RoutingTable<NM, CM, D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<NM: NodeManager, CM: ContactManager, D: Distance<NM, CM>> RoutingTable<NM, CM, D> {
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
            _phantom_distance: PhantomData,
        }
    }
}

impl<NM: NodeManager, CM: ContactManager, D: Distance<NM, CM>> RouteStorage<NM, CM>
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
    /// - `route`: The `Route<NM, CM>` to be stored.
    fn store(&mut self, bundle: &Bundle, route: Route<NM, CM>) {
        let dest = bundle.destinations[0];
        if self.tables.len() < 1 + dest as usize {
            self.tables.resize((dest + 1) as usize, vec![])
        }
        self.tables[dest as usize].push(route);
    }

    /// Selects the best route for a bundle, based on current network conditions and
    /// the `Distance<NM, CM>` trait.
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
    /// - `Some(Route<NM, CM>)` if a suitable route is found.
    /// - `None` if no feasible route is available.
    fn select(
        &mut self,
        bundle: &Bundle,
        curr_time: crate::types::Date,
        multigraph: Rc<RefCell<Multigraph<NM, CM>>>,
        excluded_nodes_sorted: &[NodeID],
    ) -> Result<Option<Route<NM, CM>>, ASABRError> {
        let dest = bundle.destinations[0];

        if self.tables.len() < 1 + dest as usize {
            self.tables.resize((dest + 1) as usize, vec![])
        }

        let routes = &mut self.tables[dest as usize];
        let mut best_route_option: Option<Route<NM, CM>> = None;

        let mut i = 0;
        while i < routes.len() {
            let should_remove = {
                let route = &routes[i];

                if curr_time > route.destination_stage.borrow().expiration {
                    true
                } else {
                    // apply exclusions
                    multigraph
                        .try_borrow_mut()?
                        .prepare_for_exclusions_sorted(excluded_nodes_sorted)?;

                    // dry run with exclusions
                    if let Some(new_candidate) =
                        dry_run_unicast_path(bundle, curr_time, route.source_stage.clone(), true)?
                    {
                        match best_route_option {
                            Some(ref best_route) => {
                                if D::cmp(
                                    &new_candidate.borrow(),
                                    &best_route.destination_stage.borrow(),
                                ) == Ordering::Less
                                {
                                    best_route_option = Some(route.clone());
                                }
                            }
                            None => {
                                best_route_option = Some(route.clone());
                            }
                        }
                    }
                    false
                }
            }; // All borrows dropped here

            if should_remove {
                routes.remove(i);
            } else {
                i += 1;
            }
        }

        Ok(best_route_option)
    }
}
