use std::{
    cell::RefCell,
    cmp::{Ordering, Reverse},
    collections::BinaryHeap,
    marker::PhantomData,
    rc::Rc,
};

use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    distance::{Distance, DistanceWrapper},
    errors::ASABRError,
    multigraph::Multigraph,
    node_manager::NodeManager,
    route_stage::{RouteStage, SharedRouteStage},
    types::{Date, NodeID},
};

/// A trait that allows HybridParenting to handle the lexicographic costs.
///
/// # Type Parameters
/// - `CM`: A type that implements the `ContactManager` trait, representing the contact management
///   system used to manage and compare routes.
pub trait HybridParentingOrd<NM, CM>
where
    NM: NodeManager,
    CM: ContactManager,
{
    /// Determines whether the proposed route stage can be retained based on the known route stage.
    /// For example, in SABR's case, a route proposal might still be part of the end-to-end route for another
    /// destination if its hop count is lower than the known route's, even if the proposal has a later arrival time.
    ///
    /// # Parameters
    /// - `prop`: A reference to the proposed `RouteStage`. This represents the current state being evaluated for retention.
    /// - `known`: A reference to the known `RouteStage`. This represents the baseline or reference state for comparison.
    ///
    /// # Returns
    /// - `true` if the `prop` can be retained considering the `known` route stage.
    /// - `false` otherwise.
    fn can_retain(prop: &RouteStage<NM, CM>, known: &RouteStage<NM, CM>) -> bool;

    /// Determines whether the known route should be pruned due to the proposition's retention.
    ///
    /// # Parameters
    /// - `prop`: A reference to the proposed `RouteStage`. This represents the proposition that was retained.
    /// - `known`: A reference to the known `RouteStage`. This represents the candidate for pruning.
    ///
    /// # Returns
    /// - `true` if the `known` can be pruned considering the `prop` route stage.
    /// - `false` otherwise.
    fn must_prune(prop: &RouteStage<NM, CM>, known: &RouteStage<NM, CM>) -> bool;
}

/// A structure representing a work area for multi-path tracking (MPT) pathfinding.
///
/// `HybridParentingWorkArea` maintains information about the current routing state, including
/// the initial bundle, the source route stage, excluded nodes, and routes grouped by destination.
/// This structure is used in pathfinding operations to manage and organize route stages for
/// efficient routing in a multi-destination network.
///
/// This type is designed to derive easily a PathFindingOutput from this work area.
///
/// # Type Parameters
/// - `NM`: A type implementing the `NodeManager` trait.
/// - `CM`: A type implementing the `ContactManager` trait, which handles contacts for routing.
struct HybridParentingWorkArea<NM: NodeManager, CM: ContactManager> {
    /// The bundle associated with this work area.
    pub bundle: Bundle,
    /// The source route stage, representing the starting point for routing.
    pub source: SharedRouteStage<NM, CM>,
    /// A sorted list of node IDs to be excluded from routing paths.
    pub excluded_nodes_sorted: Vec<NodeID>,
    /// A vector containing vectors of route stages, grouped by destination.
    /// Each inner vector represents possible routes to a specific destination,
    /// sorted in order of preference.
    pub by_destination: Vec<Vec<SharedRouteStage<NM, CM>>>,
}

impl<NM: NodeManager, CM: ContactManager> HybridParentingWorkArea<NM, CM> {
    /// Creates a new `HybridParentingWorkArea` instance, initializing it with the given bundle,
    /// source route, excluded nodes, and a specified number of destination nodes.
    ///
    /// # Parameters
    /// - `bundle`: A reference to the `Bundle` representing the data payload for routing.
    /// - `source`: A `SharedRouteStage<NM, CM>` reference to the initial route stage.
    /// - `excluded_nodes_sorted`: A reference to a sorted vector of `NodeID`s to be excluded from routing paths.
    /// - `node_count`: The number of destination nodes, which determines the size of `by_destination`.
    ///
    /// # Returns
    /// A new instance of `HybridParentingWorkArea` initialized with the provided parameters.
    pub fn new(
        bundle: &Bundle,
        source: SharedRouteStage<NM, CM>,
        excluded_nodes_sorted: &[NodeID],
        node_count: usize,
    ) -> Self {
        let exclusions = excluded_nodes_sorted.to_owned();
        Self {
            bundle: bundle.clone(),
            source,
            excluded_nodes_sorted: exclusions,
            by_destination: vec![Vec::new(); node_count],
        }
    }

    /// Converts this `HybridParentingWorkArea` into a `PathFindingOutput`, organizing routes for each destination.
    ///
    /// This function creates a `PathFindingOutput` by selecting the preferred route (if any) for each
    /// destination in `by_destination`. For each destination, if a route exists, it is added to the output;
    /// otherwise, `None` is added to indicate no viable route.
    ///
    /// # Returns
    /// A `PathFindingOutput<NM, CM>` containing the bundle, source route stage, excluded nodes,
    /// and selected routes by destination.
    pub fn into_pathfinding_output(self) -> PathFindingOutput<NM, CM> {
        let mut options = Vec::new();

        for routes in &self.by_destination {
            if routes.is_empty() {
                options.push(None);
            } else {
                options.push(Some(routes[0].clone()));
            }
        }

        PathFindingOutput {
            bundle: self.bundle,
            source: self.source,
            excluded_nodes_sorted: self.excluded_nodes_sorted.clone(),
            by_destination: options,
        }
    }
}

use super::{PathFindingOutput, Pathfinding, try_make_hop};

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
/// * `Option<SharedRouteStage<NM, CM>>` - Returns an `Option` containing a reference to the
///   newly inserted route if the insertion was successful; otherwise, returns `None`.
fn try_insert<
    NM: NodeManager,
    CM: ContactManager,
    D: Distance<NM, CM> + HybridParentingOrd<NM, CM>,
>(
    proposition: RouteStage<NM, CM>,
    tree: &mut HybridParentingWorkArea<NM, CM>,
) -> Result<Option<SharedRouteStage<NM, CM>>, ASABRError> {
    let routes_for_rx_node = &mut tree.by_destination[proposition.to_node as usize];
    // if D::can_retain sets insert to true, but the next element does not trigger insert_index =idx, insert at the end
    let mut insert_index: usize = routes_for_rx_node.len();
    let mut insert = false;

    if routes_for_rx_node.is_empty() {
        let proposition_rc = Rc::new(RefCell::new(proposition));
        routes_for_rx_node.push(Rc::clone(&proposition_rc));
        return Ok(Some(proposition_rc));
    }

    for (idx, route) in routes_for_rx_node.iter().enumerate() {
        let route_borrowed = route.borrow();
        match D::cmp(&proposition, &route_borrowed) {
            Ordering::Less => {
                // If we reached a positive can_retain call on the previous element
                insert_index = idx;
                insert = true;
                break;
            }
            Ordering::Equal => {
                insert = false;
                break;
            }
            Ordering::Greater => {
                if D::can_retain(&proposition, &route_borrowed) {
                    insert = true;
                    continue;
                } else {
                    insert = false;
                    break;
                }
            }
        }
    }
    if insert {
        let mut truncate_index = insert_index;
        // detect the first prune event but do nothing
        while truncate_index < routes_for_rx_node.len() {
            let route = &routes_for_rx_node[truncate_index].borrow();
            if D::must_prune(&proposition, route) {
                break;
            }
            truncate_index += 1
        }

        // Now disable the routes(for the shared ref in the priority queue)
        for route in routes_for_rx_node.iter().skip(truncate_index) {
            route.try_borrow_mut()?.is_disabled = true;
        }

        // Now truncate
        routes_for_rx_node.truncate(truncate_index);

        let proposition_rc = Rc::new(RefCell::new(proposition));
        // if everything was truncated, the following has no overhead
        routes_for_rx_node.insert(insert_index, Rc::clone(&proposition_rc));

        return Ok(Some(proposition_rc));
    }

    Ok(None)
}

macro_rules! define_mpt {
    ($name:ident, $is_tree_output:tt, $with_exclusions:tt) => {
        /// A multipath tracking (SPSN v2) implementation of Dijkstra algorithm.
        ///
        /// Use this implementation for optimized pathfinding precision.
        ///
        /// # Type Parameters
        ///
        /// * `NM` - A type that implements the `NodeManager` trait.
        /// * `CM` - A type that implements the `ContactManager` trait.
        /// * `D` - A type that implements the `Distance<NM, CM>` trait.
        pub struct $name<
            NM: NodeManager,
            CM: ContactManager,
            D: Distance<NM, CM> + HybridParentingOrd<NM, CM>,
        > {
            /// The node multigraph for contact access.
            graph: Rc<RefCell<Multigraph<NM, CM>>>,
            #[doc(hidden)]
            _phantom_distance: PhantomData<D>,
        }

        impl<NM: NodeManager, CM: ContactManager, D: Distance<NM, CM> + HybridParentingOrd<NM, CM>>
            Pathfinding<NM, CM> for $name<NM, CM, D>
        {
            /// Constructs a new `HybridParenting` instance with the provided nodes and contacts.
            ///
            /// # Parameters
            ///
            /// * `multigraph` - A shared pointer to a multigraph.
            ///
            /// # Returns
            ///
            #[doc = concat!( " * `Self` - A new instance of `",stringify!($name),"`.")]
            fn new(multigraph: Rc<RefCell<Multigraph<NM, CM>>>) -> Self {
                Self {
                    graph: multigraph,
                    _phantom_distance: PhantomData,
                }
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
            /// * `excluded_nodes_sorted` - A sorted list of `NodeID`s to be excluded from the pathfinding.
            ///
            /// # Returns
            ///
            /// * `<ResultPathFindingOutput<NM, CM>, ASABRError>` - The resulting pathfinding output, including the routes found,
            ///   or an error if the operation fails.
            fn get_next(
                &mut self,
                current_time: Date,
                source: NodeID,
                bundle: &Bundle,
                excluded_nodes_sorted: &[NodeID],
            ) -> Result<PathFindingOutput<NM, CM>, ASABRError> {
                let mut graph = self.graph.try_borrow_mut()?;
                if $with_exclusions {
                    graph.prepare_for_exclusions_sorted(excluded_nodes_sorted)?;
                }
                let source_route: SharedRouteStage<NM, CM> =
                    Rc::new(RefCell::new(RouteStage::new(
                        current_time,
                        source,
                        None,
                        #[cfg(feature = "node_proc")]
                        bundle.clone(),
                    )));
                let mut tree: HybridParentingWorkArea<NM, CM> = HybridParentingWorkArea::new(
                    bundle,
                    source_route.clone(),
                    excluded_nodes_sorted,
                    graph.get_vertex_count(),
                );
                let mut priority_queue: BinaryHeap<Reverse<DistanceWrapper<NM, CM, D>>> =
                    BinaryHeap::new();

                tree.by_destination[source as usize].push(source_route.clone());
                priority_queue.push(Reverse(DistanceWrapper::new(Rc::clone(&source_route))));

                while let Some(Reverse(DistanceWrapper(from_route, _))) = priority_queue.pop() {
                    if from_route.borrow().is_disabled {
                        continue;
                    }

                    let tx_node_id = from_route.borrow().to_node;

                    if !$is_tree_output {
                        if bundle.destinations[0] == tx_node_id {
                            break;
                        }
                    }

                    let sender = &graph.senders[tx_node_id as usize];

                    for receiver in &sender.receivers {
                        if $with_exclusions {
                            if receiver.is_excluded(&graph.real_nodes) {
                                continue;
                            }
                        }

                        if let Some(first_contact_index) =
                            receiver.lazy_prune_and_get_first_idx(current_time)
                            && let Some(route_proposition) = try_make_hop(
                                first_contact_index,
                                &from_route,
                                bundle,
                                receiver.vertex_id,
                                &receiver.contacts_to_receiver,
                                &graph.real_nodes,
                            )
                            // This transforms a prop in the stack to a prop in the heap
                            && let Some(new_route) =
                                try_insert::<NM, CM, D>(route_proposition, &mut tree)?
                        {
                            priority_queue.push(Reverse(DistanceWrapper::new(new_route.clone())));
                        }
                    }
                }

                // totally fine as we have Rcs
                for v in &mut tree.by_destination {
                    v.truncate(1);
                }

                return Ok(tree.into_pathfinding_output());
            }

            /// Get a shared pointer to the multigraph.
            ///
            /// # Returns
            ///
            /// * A shared pointer to the multigraph.
            fn get_multigraph(&self) -> Rc<RefCell<Multigraph<NM, CM>>> {
                return self.graph.clone();
            }
        }
    };
}

define_mpt!(HybridParentingTreeExcl, true, true);
define_mpt!(HybridParentingPath, false, false);
define_mpt!(HybridParentingPathExcl, false, true);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contact_manager::legacy::evl::EVLManager;
    use crate::distance::hop::Hop;
    use crate::distance::sabr::SABR;
    use crate::node_manager::none::NoManagement;
    use crate::pathfinding::ASABRError;
    use crate::pathfinding::test_helpers::*;

    #[test]
    fn test_a_to_c_tree() -> Result<(), ASABRError> {
        let mg = unit_graph_test()?;

        let mut algo_hop =
            HybridParentingTreeExcl::<NoManagement, EVLManager, Hop>::new(mg.clone());
        let mut algo_sabr =
            HybridParentingTreeExcl::<NoManagement, EVLManager, SABR>::new(mg.clone());

        let bundle = make_bundle(2, 1, 1.0, 2000.0);

        let res_hop = algo_hop
            .get_next(0.0, 0, &bundle, &[][..])
            .expect("Hop : Routing Failed !");

        assert_time_hop(&res_hop, 2, 2.02, 2, "Hop");

        let res_sabr = algo_sabr
            .get_next(0.0, 0, &bundle, &[][..])
            .expect("SABR : Routing Failed !");

        assert_time_hop(&res_sabr, 2, 2.02, 2, "SABR");

        Ok(())
    }

    #[test]
    fn test_a_to_c_tree_excluded() -> Result<(), ASABRError> {
        let mg = unit_graph_test()?;

        let mut algo_hop =
            HybridParentingTreeExcl::<NoManagement, EVLManager, Hop>::new(mg.clone());
        let mut algo_sabr =
            HybridParentingTreeExcl::<NoManagement, EVLManager, SABR>::new(mg.clone());

        let bundle = make_bundle(2, 1, 1.0, 2000.0);
        let excluded = [1];

        let res_hop = algo_hop
            .get_next(0.0, 0, &bundle, &excluded[..])
            .expect("Hop : Routing Failed !");
        assert!(
            res_hop.by_destination[1].is_none(),
            "Hop : B should be excluded"
        );
        assert!(
            res_hop.by_destination[2].is_none(),
            "Hop : C should not be accessible without B"
        );

        let res_sabr = algo_sabr
            .get_next(0.0, 0, &bundle, &excluded[..])
            .expect("SABR : Routing Failed !");
        assert!(
            res_sabr.by_destination[1].is_none(),
            "SABR : B should be excluded"
        );
        assert!(
            res_sabr.by_destination[2].is_none(),
            "SABR : C should not be accessible without B"
        );

        Ok(())
    }

    #[test]
    fn test_a_to_c_path_excl() -> Result<(), ASABRError> {
        let mg = unit_graph_test()?;

        let mut algo_hop =
            HybridParentingPathExcl::<NoManagement, EVLManager, Hop>::new(mg.clone());
        let mut algo_sabr =
            HybridParentingPathExcl::<NoManagement, EVLManager, SABR>::new(mg.clone());

        let bundle = make_bundle(2, 1, 1.0, 2000.0);
        let excluded = [1];

        let res_hop = algo_hop
            .get_next(0.0, 0, &bundle, &excluded[..])
            .expect("Hop : Routing Failed !");
        assert!(
            res_hop.by_destination[2].is_none(),
            "Hop : C should not be accessible without B"
        );

        let res_sabr = algo_sabr
            .get_next(0.0, 0, &bundle, &excluded[..])
            .expect("SABR : Routing Failed !");
        assert!(
            res_sabr.by_destination[2].is_none(),
            "SABR : C should not be accessible without B"
        );

        Ok(())
    }

    #[test]
    fn test_two_paths_to_c() -> Result<(), ASABRError> {
        let mg = five_contact_graph_test()?;

        let mut algo_hop =
            HybridParentingTreeExcl::<NoManagement, EVLManager, Hop>::new(mg.clone());
        let mut algo_sabr =
            HybridParentingTreeExcl::<NoManagement, EVLManager, SABR>::new(mg.clone());

        let bundle = make_bundle(2, 1, 1.0, 2000.0);

        let res_hop = algo_hop
            .get_next(0.0, 0, &bundle, &[][..])
            .expect("Hop : Routing Failed !");

        assert_time_hop(&res_hop, 2, 10.01, 1, "Hop");

        let res_sabr = algo_sabr
            .get_next(0.0, 0, &bundle, &[][..])
            .expect("SABR : Routing Failed !");

        assert_time_hop(&res_sabr, 2, 0.13, 2, "SABR");

        Ok(())
    }

    #[test]
    fn test_exemple_1() -> Result<(), ASABRError> {
        let mg = exemple_1_graph()?;

        let mut algo_hop = HybridParentingPath::<NoManagement, EVLManager, Hop>::new(mg.clone());
        let mut algo_sabr = HybridParentingPath::<NoManagement, EVLManager, SABR>::new(mg.clone());

        let bundle = make_bundle(3, 0, 0.0, 1000.0);

        let res_hop = algo_hop
            .get_next(0.0, 0, &bundle, &[][..])
            .expect("Routing Failed !");

        assert_time_hop(&res_hop, 3, 30.0, 2, "Hop");

        let res_sabr = algo_sabr
            .get_next(0.0, 0, &bundle, &[][..])
            .expect("Routing Failed !");

        assert_time_hop(&res_sabr, 3, 30.0, 2, "SABR");

        Ok(())
    }

    #[test]
    fn test_exemple_2() -> Result<(), ASABRError> {
        let mg = exemple_2_graph()?;

        let mut algo_hop = HybridParentingPath::<NoManagement, EVLManager, Hop>::new(mg.clone());
        let mut algo_sabr = HybridParentingPath::<NoManagement, EVLManager, SABR>::new(mg.clone());

        let bundle = make_bundle(4, 0, 0.0, 1000.0);

        let res_hop = algo_hop
            .get_next(0.0, 0, &bundle, &[][..])
            .expect("Routing Failed !");

        assert_time_hop(&res_hop, 4, 50.0, 3, "Hop");

        let res_sabr = algo_sabr
            .get_next(0.0, 0, &bundle, &[][..])
            .expect("Routing Failed !");

        assert_time_hop(&res_sabr, 4, 50.0, 3, "SABR");

        Ok(())
    }

    #[test]
    fn test_vnode_anycast_tree() -> Result<(), ASABRError> {
        let mg = vnode_anycast_graph()?;

        let mut algo = HybridParentingTreeExcl::<NoManagement, EVLManager, SABR>::new(mg.clone());

        let bundle = make_bundle(5, 1, 1.0, 2000.0);

        let res = algo
            .get_next(0.0, 0, &bundle, &[][..])
            .expect("Routing to vnode failed!");

        assert!(
            res.by_destination[5].is_some(),
            "VNode V(5) should be reachable"
        );

        let vnode_route = res.by_destination[5].as_ref().unwrap().borrow();
        assert_eq!(
            vnode_route.to_node, 5,
            "Route to_node should be vnode vertex ID (5), got {}",
            vnode_route.to_node
        );

        Ok(())
    }

    #[test]
    fn test_vnode_anycast_path() -> Result<(), ASABRError> {
        let mg = vnode_anycast_graph()?;

        let mut algo = HybridParentingPathExcl::<NoManagement, EVLManager, SABR>::new(mg.clone());

        let bundle = make_bundle(5, 1, 1.0, 2000.0);

        let res = algo
            .get_next(0.0, 0, &bundle, &[][..])
            .expect("Routing to vnode failed!");

        assert!(
            res.by_destination[5].is_some(),
            "VNode V(5) should be reachable via path search"
        );

        let vnode_route = res.by_destination[5].as_ref().unwrap().borrow();
        assert_eq!(
            vnode_route.to_node, 5,
            "Route to_node should be vnode ID (5)"
        );

        Ok(())
    }
}
