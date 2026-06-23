extern crate alloc;
use alloc::{vec, vec::Vec};

use core::marker::PhantomData;

use super::{PathFindingOutput, Pathfinding, try_make_hop};
use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    distance::{Distance, prio_queue::PrioQueue},
    errors::ASABRError,
    multigraph::{Multigraph, NodeRef, RNodeRef},
    node_manager::NodeManager,
    parsing::Either,
    paths::PathFragment,
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
    /// Wether both Path should be kept as potential candidate.
    fn keep_both<'id>(
        first: &PathFragment<'id>,
        second: &PathFragment<'id>,
        graph: &Multigraph<'id, NM, CM>,
        bundle: &Bundle,
        actual_node: RNodeRef<'id>,
    ) -> bool;
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
struct HybridParentingWorkArea<
    'id,
    'a,
    NM: NodeManager,
    CM: ContactManager,
    D: HybridParentingOrd<NM, CM>,
> {
    /// The bundle associated with this work area.
    bundle: &'a Bundle,
    /// A vector storing all keeped path to a node without sorting for easy reference.
    possible_paths: Vec<PathFragment<'id>>,
    /// A vector containing vectors of (index in possible path of) pathfragment, grouped by destination.
    /// Each inner vector represents possible routes to a specific destination,
    /// sorted in order of preference.
    by_destination: Vec<Vec<usize>>,
    _phantom: PhantomData<fn(NM, CM, D)>,
}

impl<'id, 'a, NM: NodeManager, CM: ContactManager, D: Distance<NM, CM> + HybridParentingOrd<NM, CM>>
    HybridParentingWorkArea<'id, 'a, NM, CM, D>
{
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
    pub fn new(bundle: &'a Bundle, _source: NodeRef<'id>, graph: &Multigraph<'id, NM, CM>) -> Self {
        Self {
            bundle,
            possible_paths: Vec::new(),
            by_destination: vec![Vec::new(); graph.get_rnode_count()],
            _phantom: PhantomData,
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
    pub fn into_pathfinding_output<'b>(self) -> PathFindingOutput<'id, 'b> {
        // TODO: Better heuristic maybe ?
        let mut elided_tree =
            Vec::with_capacity(self.by_destination.len().max(self.possible_paths.len() / 3));
        let mut new_indexs = vec![None; self.possible_paths.len()];
        for (i, possible_path) in self.by_destination.iter().enumerate() {
            let path = possible_path.first().map(|index| {
                new_indexs[*index] = Some(i);
                self.possible_paths[*index]
            });
            elided_tree.push(path);
        }

        for i in 0..self.by_destination.len() {
            if let Some(mut frag) = elided_tree[i] {
                let mut index = i;
                loop {
                    if let Some(via) = frag.via.as_mut() {
                        if let Some(new_idx) = new_indexs[via.parent_frag] {
                            via.parent_frag = new_idx;
                            elided_tree[index] = Some(frag);
                            break;
                        } else {
                            let old_idx = via.parent_frag;
                            let new_idx = elided_tree.len();
                            elided_tree.push(None);
                            via.parent_frag = new_idx;
                            frag = self.possible_paths[old_idx];
                            index = new_idx;
                        }
                    } else {
                        elided_tree[index] = Some(frag);
                        break;
                    }
                }
            } else {
                continue;
            }
        }

        PathFindingOutput {
            path_tree: Either::Right(elided_tree),
        }
    }

    /// Attempts to insert a new route proposal into the pathfinding output tree.
    ///
    /// This function checks if the proposed route is strictly or partially better than existing
    /// routes for the specified receiver node. If it is better, the function updates the routes
    /// accordingly and disables less favorable routes.
    ///
    /// # Parameters
    ///
    /// * `proposition` - The `RouteStage` representing the new route proposal.
    ///
    /// # Returns
    ///
    /// * `Option<usize>` - Returns an `Option` containing a reference(index) to the
    ///   newly inserted route if the insertion was successful; otherwise, returns `None`.
    fn try_insert(
        &mut self,
        proposition: PathFragment<'id>,
        actual_node: RNodeRef<'id>,
        graph: &Multigraph<'id, NM, CM>,
    ) -> Option<usize> {
        let new_idx = self.possible_paths.len();
        let routes_for_node = &mut self.by_destination[NodeID::from(actual_node) as usize];
        if routes_for_node.iter().all(|path| {
            D::keep_both(
                &proposition,
                &self.possible_paths[*path],
                graph,
                self.bundle,
                actual_node,
            )
        }) {
            routes_for_node.push(new_idx);
            self.possible_paths.push(proposition);
            Some(new_idx)
        } else {
            None
        }
    }
    /// return true iif this path proposition merit being inserted in disktra priority queue
    fn check_insert(
        &self,
        proposition: PathFragment<'id>,
        dest_node: RNodeRef<'id>,
        graph: &Multigraph<'id, NM, CM>,
    ) -> bool {
        let routes_for_node = &self.by_destination[NodeID::from(dest_node) as usize];
        routes_for_node.iter().all(|path| {
            D::keep_both(
                &proposition,
                &self.possible_paths[*path],
                graph,
                self.bundle,
                dest_node,
            )
        })
    }
}
/// A multipath tracking (SPSN v2) implementation of Dijkstra algorithm.
///
/// Use this implementation for optimized pathfinding precision.
///
/// # Type Parameters
///
/// * `NM` - A type that implements the `NodeManager` trait.
/// * `CM` - A type that implements the `ContactManager` trait.
/// * `D` - A type that implements the `Distance<NM, CM>` trait.
pub struct HybridParenting<
    'id,
    const is_tree_output: bool,
    NM: NodeManager,
    CM: ContactManager,
    D: Distance<NM, CM> + HybridParentingOrd<NM, CM>,
> {
    #[doc(hidden)]
    _phantom: PhantomData<fn(&'id (D, NM, CM))>,
}

impl<
    'id,
    const tree_output: bool,
    NM: NodeManager,
    CM: ContactManager,
    D: Distance<NM, CM> + HybridParentingOrd<NM, CM>,
> Pathfinding<'id, NM, CM> for HybridParenting<'id, tree_output, NM, CM, D>
{
    /// Constructs a new `HybridParenting` instance suitable to work with the provided multigraph
    fn new(_id: generativity::Guard, _multigraph: &Multigraph<'id, NM, CM>) -> Self {
        Self {
            _phantom: PhantomData,
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
    fn find_path<'a>(
        &'a mut self,
        multigraph: &mut Multigraph<'id, NM, CM>,
        current_time: Date,
        source: NodeRef<'id>,
        bundle: &Bundle,
    ) -> Result<Option<PathFindingOutput<'id, 'a>>, ASABRError> {
        // Common variables in a area to delegate a few function on it, see above
        let mut work_area = HybridParentingWorkArea::<NM, CM, D>::new(bundle, source, multigraph);
        // The priority queue
        let mut prioqueue =
            PrioQueue::<'_, D, NM, CM, ()>::with_capacity(multigraph.get_rnode_count());

        // Used only when tree_output = true, defined outside of if scope but never used
        let mut reachable: usize = 0;
        let mut reached: usize = 0;
        let start = PathFragment::new_start(current_time);
        for rnode in multigraph.iter_node(source) {
            prioqueue.insert(((start, rnode), ()), multigraph, bundle);
            if tree_output {
                reachable += 1;
            }
        }

        while (if tree_output {
            reachable > reached
        } else {
            // TODO: support vnode outputs
            work_area.by_destination[bundle.destinations[0] as usize].is_empty()
        }) && let Some(((path, node), ())) = prioqueue.pop_min(multigraph, bundle)
        {
            if tree_output && work_area.by_destination[NodeID::from(node) as usize].is_empty() {
                reached += 1;
            }
            if let Some(viaref) = work_area.try_insert(path, node, multigraph) {
                let (current_node, iter) = multigraph.iter_iter_contacts(node);
                for (neighbor, _, contacts) in iter {
                    let delay = current_node.manager.delay(
                        bundle,
                        path.arrival_time,
                        node.into(),
                        neighbor.into(),
                    );
                    if let Some(path) = try_make_hop(
                        multigraph,
                        (&path, viaref),
                        bundle,
                        node,
                        neighbor,
                        delay,
                        contacts,
                    ) {
                        if tree_output
                            && work_area.by_destination[NodeID::from(neighbor) as usize].is_empty()
                        {
                            reachable += 1
                        }
                        if work_area.check_insert(path, neighbor, multigraph) {
                            prioqueue.insert(((path, neighbor), ()), multigraph, bundle);
                        }
                    }
                }
            }
        }

        Ok(Some(work_area.into_pathfinding_output()))
    }
}

#[cfg(test)]
mod tests {
    use generativity::make_guard;

    use super::*;
    use crate::contact_manager::legacy::evl::EVLManager;
    use crate::distance::hop::Hop;
    use crate::distance::sabr::SABR;
    use crate::node_manager::none::NoManagement;
    use crate::pathfinding::ASABRError;
    use crate::pathfinding::test_helpers::*;

    #[test]
    fn test_a_to_c_tree() -> Result<(), ASABRError> {
        for_test_graph(0, |mg, algo_hop: &mut HybridParenting<true, _, _, Hop>| {
            make_guard!(guard);
            let mut algo_sabr =
                HybridParenting::<true, NoManagement, EVLManager, SABR>::new(guard, mg);

            let bundle = make_bundle(2, 1, 1.0, 2000);

            let res_hop = algo_hop
                .find_path(mg, 0, mg.node_id_ref(0).unwrap(), &bundle)
                .expect("Hop : Routing Failed !")
                .unwrap();

            assert_time_hop(&res_hop, 2, 2, 2, "Hop");

            let res_sabr = algo_sabr
                .find_path(mg, 0, mg.node_id_ref(0).unwrap(), &bundle)
                .expect("Hop : Routing Failed !")
                .unwrap();

            assert_time_hop(&res_sabr, 2, 2, 2, "SABR");

            Ok(())
        })
    }

    #[test]
    fn test_a_to_c_tree_excluded() -> Result<(), ASABRError> {
        for_test_graph(0, |mg, algo_hop: &mut HybridParenting<true, _, _, Hop>| {
            make_guard!(guard);
            let mut algo_sabr =
                HybridParenting::<true, NoManagement, EVLManager, SABR>::new(guard, mg);

            let bundle = make_bundle(2, 1, 1.0, 2000);
            let excluded = [1].map(|id| mg.node_id_ref(id).unwrap().real().unwrap());
            mg.mark_excluded(&excluded);
            let res_hop = algo_hop
                .find_path(mg, 0, mg.node_id_ref(0).unwrap(), &bundle)
                .expect("Hop : Routing Failed !")
                .unwrap();
            assert!(res_hop[1].is_none(), "Hop : B should be excluded");
            assert!(
                res_hop[2].is_none(),
                "Hop : C should not be accessible without B"
            );

            let res_sabr = algo_sabr
                .find_path(mg, 0, mg.node_id_ref(0).unwrap(), &bundle)
                .expect("Hop : Routing Failed !")
                .unwrap();
            assert!(res_sabr[1].is_none(), "SABR : B should be excluded");
            assert!(
                res_sabr[2].is_none(),
                "SABR : C should not be accessible without B"
            );

            Ok(())
        })
    }

    #[test]
    fn test_a_to_c_path_excl() -> Result<(), ASABRError> {
        for_test_graph(0, |mg, algo_hop: &mut HybridParenting<false, _, _, Hop>| {
            make_guard!(guard);
            let mut algo_sabr =
                HybridParenting::<false, NoManagement, EVLManager, SABR>::new(guard, mg);

            let bundle = make_bundle(2, 1, 1.0, 2000);
            let excluded = [1].map(|id| mg.node_id_ref(id).unwrap().real().unwrap());
            mg.mark_excluded(&excluded);

            let res_hop = algo_hop
                .find_path(mg, 0, mg.node_id_ref(0).unwrap(), &bundle)
                .expect("Hop : Routing Failed !")
                .unwrap();
            assert!(
                res_hop[2].is_none(),
                "Hop : C should not be accessible without B"
            );

            let res_sabr = algo_sabr
                .find_path(mg, 0, mg.node_id_ref(0).unwrap(), &bundle)
                .expect("Hop : Routing Failed !")
                .unwrap();
            assert!(
                res_sabr[2].is_none(),
                "SABR : C should not be accessible without B"
            );

            Ok(())
        })
    }

    #[test]
    fn test_two_paths_to_c() -> Result<(), ASABRError> {
        for_test_graph(1, |mg, algo_hop: &mut HybridParenting<false, _, _, Hop>| {
            make_guard!(guard);
            let mut algo_sabr =
                HybridParenting::<false, NoManagement, EVLManager, SABR>::new(guard, mg);

            let bundle = make_bundle(2, 1, 1.0, 2000);

            let res_hop = algo_hop
                .find_path(mg, 0, mg.node_id_ref(0).unwrap(), &bundle)
                .expect("Hop : Routing Failed !")
                .unwrap();

            assert_time_hop(&res_hop, 2, 11, 1, "Hop");

            let res_sabr = algo_sabr
                .find_path(mg, 0, mg.node_id_ref(0).unwrap(), &bundle)
                .expect("Hop : Routing Failed !")
                .unwrap();

            assert_time_hop(&res_sabr, 2, 3, 2, "SABR");

            Ok(())
        })
    }

    #[test]
    fn test_exemple_1() -> Result<(), ASABRError> {
        for_test_graph(2, |mg, algo_hop: &mut HybridParenting<false, _, _, Hop>| {
            make_guard!(guard);
            let mut algo_sabr =
                HybridParenting::<false, NoManagement, EVLManager, SABR>::new(guard, mg);

            let bundle = make_bundle(3, 0, 0.0, 1000);

            let res_hop = algo_hop
                .find_path(mg, 0, mg.node_id_ref(0).unwrap(), &bundle)
                .expect("Hop : Routing Failed !")
                .unwrap();

            assert_time_hop(&res_hop, 3, 30, 2, "Hop");

            let res_sabr = algo_sabr
                .find_path(mg, 0, mg.node_id_ref(0).unwrap(), &bundle)
                .expect("Hop : Routing Failed !")
                .unwrap();

            assert_time_hop(&res_sabr, 3, 30, 2, "SABR");

            Ok(())
        })
    }

    #[test]
    fn test_exemple_2() -> Result<(), ASABRError> {
        for_test_graph(3, |mg, algo_hop: &mut HybridParenting<false, _, _, Hop>| {
            make_guard!(guard);
            let mut algo_sabr =
                HybridParenting::<false, NoManagement, EVLManager, SABR>::new(guard, mg);

            let bundle = make_bundle(4, 0, 0.0, 1000);

            let res_hop = algo_hop
                .find_path(mg, 0, mg.node_id_ref(0).unwrap(), &bundle)
                .expect("Hop : Routing Failed !")
                .unwrap();

            assert_time_hop(&res_hop, 4, 50, 3, "Hop");

            let res_sabr = algo_sabr
                .find_path(mg, 0, mg.node_id_ref(0).unwrap(), &bundle)
                .expect("Hop : Routing Failed !")
                .unwrap();

            assert_time_hop(&res_sabr, 4, 50, 3, "SABR");

            Ok(())
        })
    }

    // #[test]
    // fn test_vnode_anycast_tree() -> Result<(), ASABRError> {
    //     let mg = vnode_anycast_graph()?;

    //     let mut algo = HybridParentingTreeExcl::<NoManagement, EVLManager, SABR>::new(mg.clone());

    //     let bundle = make_bundle(5, 1, 1.0, 2000.0);

    //     let res = algo
    //         .get_next(0.0, 0, &bundle, &[][..])
    //         .expect("Routing to vnode failed!");

    //     assert!(
    //         res.by_destination[5].is_some(),
    //         "VNode V(5) should be reachable"
    //     );

    //     let vnode_route = res.by_destination[5].as_ref().unwrap().borrow();
    //     assert_eq!(
    //         vnode_route.to_node, 5,
    //         "Route to_node should be vnode vertex ID (5), got {}",
    //         vnode_route.to_node
    //     );

    //     Ok(())
    // }

    // #[test]
    // fn test_vnode_anycast_path() -> Result<(), ASABRError> {
    //     let mg = vnode_anycast_graph()?;

    //     let mut algo = HybridParentingPathExcl::<NoManagement, EVLManager, SABR>::new(mg.clone());

    //     let bundle = make_bundle(5, 1, 1.0, 2000.0);

    //     let res = algo
    //         .get_next(0.0, 0, &bundle, &[][..])
    //         .expect("Routing to vnode failed!");

    //     assert!(
    //         res.by_destination[5].is_some(),
    //         "VNode V(5) should be reachable via path search"
    //     );

    //     let vnode_route = res.by_destination[5].as_ref().unwrap().borrow();
    //     assert_eq!(
    //         vnode_route.to_node, 5,
    //         "Route to_node should be vnode ID (5)"
    //     );

    //     Ok(())
    // }
}
