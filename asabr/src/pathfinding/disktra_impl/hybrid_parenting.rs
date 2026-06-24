extern crate alloc;
use alloc::{vec, vec::Vec};

use core::marker::PhantomData;

use super::super::{PathFindingOutput, Pathfinding};
use crate::{
    bundle::Bundle, contact_manager::ContactManager, distance::Distance, multigraph::{Multigraph, RNodeRef}, node_manager::NodeManager, pathfinding::{disktra::{Disktra, DisktraWorkspace}, flatten}, paths::PathFragment, types::NodeID
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
    NM: NodeManager,
    CM: ContactManager,
    D: HybridParentingOrd<NM, CM>,
> {
    /// A vector storing all keeped path to a node without sorting for easy reference.
    possible_paths: Vec<PathFragment<'id>>,
    /// A vector containing vectors of (index in possible path of) pathfragment, grouped by destination.
    /// Each inner vector represents possible routes to a specific destination,
    /// sorted in order of preference.
    by_destination: Vec<Vec<usize>>,
    _phantom: PhantomData<fn(NM, CM, D)>,
}

impl<'id, NM: NodeManager, CM: ContactManager, D: Distance<NM, CM> + HybridParentingOrd<NM, CM>>
    DisktraWorkspace<'id,NM,CM> for
    HybridParentingWorkArea<'id, NM, CM, D>
{
    #[inline(always)]
    fn new(graph: &Multigraph<'id, NM, CM>) -> Self {
        Self {
            possible_paths: Vec::new(),
            by_destination: vec![Vec::new(); graph.get_rnode_count()],
            _phantom: PhantomData,
        }
    }

    fn into_pathfinding_output(self) -> PathFindingOutput<'id, 'static> {
        flatten(&self.possible_paths, self.by_destination.into_iter().map(|possibilities| possibilities.first()))
    }

    fn try_insert(
        &mut self,
        proposition: PathFragment<'id>,
        actual_node: RNodeRef<'id>,
        graph: &Multigraph<'id, NM, CM>,
        bundle: &Bundle
    ) -> (Option<usize>,bool) {
        let new_idx = self.possible_paths.len();
        let routes_for_node = &mut self.by_destination[NodeID::from(actual_node) as usize];
        if routes_for_node.iter().all(|path| {
            D::keep_both(
                &proposition,
                &self.possible_paths[*path],
                graph,
                bundle,
                actual_node,
            )
        }) {
            routes_for_node.push(new_idx);
            self.possible_paths.push(proposition);
            (Some(new_idx),routes_for_node.len() == 1)
        } else {
            (None,false)
        }
    }
    /// return true iif this path proposition merit being inserted in disktra priority queue
    #[inline(always)]
    fn fragment_check(
        &mut self,
        proposition: PathFragment<'id>,
        dest_node: RNodeRef<'id>,
        graph: &Multigraph<'id, NM, CM>,
        bundle: &Bundle,
    ) -> bool
    {
        let routes_for_node = &self.by_destination[NodeID::from(dest_node) as usize];
        routes_for_node.iter().all(|path| {
            D::keep_both(
                &proposition,
                &self.possible_paths[*path],
                graph,
                bundle,
                dest_node,
            )
        })
    }
    #[inline(always)]
    fn node_check(&mut self, node: RNodeRef<'id>) -> bool {
        true
    }
}

pub type HybridParentig<'id,D:HybridParentingOrd<NM,CM> + Distance<NM,CM>, NM:NodeManager, CM:ContactManager> = Disktra<HybridParentingWorkArea<'id,NM,CM,D>,D>;

// #[cfg(test)]
// mod tests {
//     use generativity::make_guard;

//     use super::*;
//     use crate::contact_manager::legacy::evl::EVLManager;
//     use crate::distance::hop::Hop;
//     use crate::distance::sabr::SABR;
//     use crate::node_manager::none::NoManagement;
//     use crate::pathfinding::ASABRError;
//     use crate::pathfinding::test_helpers::*;

//     #[test]
//     fn test_a_to_c_tree() -> Result<(), ASABRError> {
//         for_test_graph(0, |mg, algo_hop: &mut HybridParenting<true, _, _, Hop>| {
//             make_guard!(guard);
//             let mut algo_sabr =
//                 HybridParenting::<true, NoManagement, EVLManager, SABR>::new(guard, mg);

//             let bundle = make_bundle(2, 1, 1.0, 2000);

//             let res_hop = algo_hop
//                 .find_path(mg, 0, mg.node_id_ref(0).unwrap(), &bundle)
//                 .expect("Hop : Routing Failed !")
//                 .unwrap();

//             assert_time_hop(&res_hop, 2, 2, 2, "Hop");

//             let res_sabr = algo_sabr
//                 .find_path(mg, 0, mg.node_id_ref(0).unwrap(), &bundle)
//                 .expect("Hop : Routing Failed !")
//                 .unwrap();

//             assert_time_hop(&res_sabr, 2, 2, 2, "SABR");

//             Ok(())
//         })
//     }

//     #[test]
//     fn test_a_to_c_tree_excluded() -> Result<(), ASABRError> {
//         for_test_graph(0, |mg, algo_hop: &mut HybridParenting<true, _, _, Hop>| {
//             make_guard!(guard);
//             let mut algo_sabr =
//                 HybridParenting::<true, NoManagement, EVLManager, SABR>::new(guard, mg);

//             let bundle = make_bundle(2, 1, 1.0, 2000);
//             let excluded = [1].map(|id| mg.node_id_ref(id).unwrap().real().unwrap());
//             mg.mark_excluded(&excluded);
//             let res_hop = algo_hop
//                 .find_path(mg, 0, mg.node_id_ref(0).unwrap(), &bundle)
//                 .expect("Hop : Routing Failed !")
//                 .unwrap();
//             assert!(res_hop[1].is_none(), "Hop : B should be excluded");
//             assert!(
//                 res_hop[2].is_none(),
//                 "Hop : C should not be accessible without B"
//             );

//             let res_sabr = algo_sabr
//                 .find_path(mg, 0, mg.node_id_ref(0).unwrap(), &bundle)
//                 .expect("Hop : Routing Failed !")
//                 .unwrap();
//             assert!(res_sabr[1].is_none(), "SABR : B should be excluded");
//             assert!(
//                 res_sabr[2].is_none(),
//                 "SABR : C should not be accessible without B"
//             );

//             Ok(())
//         })
//     }

//     #[test]
//     fn test_a_to_c_path_excl() -> Result<(), ASABRError> {
//         for_test_graph(0, |mg, algo_hop: &mut HybridParenting<false, _, _, Hop>| {
//             make_guard!(guard);
//             let mut algo_sabr =
//                 HybridParenting::<false, NoManagement, EVLManager, SABR>::new(guard, mg);

//             let bundle = make_bundle(2, 1, 1.0, 2000);
//             let excluded = [1].map(|id| mg.node_id_ref(id).unwrap().real().unwrap());
//             mg.mark_excluded(&excluded);

//             let res_hop = algo_hop
//                 .find_path(mg, 0, mg.node_id_ref(0).unwrap(), &bundle)
//                 .expect("Hop : Routing Failed !")
//                 .unwrap();
//             assert!(
//                 res_hop[2].is_none(),
//                 "Hop : C should not be accessible without B"
//             );

//             let res_sabr = algo_sabr
//                 .find_path(mg, 0, mg.node_id_ref(0).unwrap(), &bundle)
//                 .expect("Hop : Routing Failed !")
//                 .unwrap();
//             assert!(
//                 res_sabr[2].is_none(),
//                 "SABR : C should not be accessible without B"
//             );

//             Ok(())
//         })
//     }

//     #[test]
//     fn test_two_paths_to_c() -> Result<(), ASABRError> {
//         for_test_graph(1, |mg, algo_hop: &mut HybridParenting<false, _, _, Hop>| {
//             make_guard!(guard);
//             let mut algo_sabr =
//                 HybridParenting::<false, NoManagement, EVLManager, SABR>::new(guard, mg);

//             let bundle = make_bundle(2, 1, 1.0, 2000);

//             let res_hop = algo_hop
//                 .find_path(mg, 0, mg.node_id_ref(0).unwrap(), &bundle)
//                 .expect("Hop : Routing Failed !")
//                 .unwrap();

//             assert_time_hop(&res_hop, 2, 11, 1, "Hop");

//             let res_sabr = algo_sabr
//                 .find_path(mg, 0, mg.node_id_ref(0).unwrap(), &bundle)
//                 .expect("Hop : Routing Failed !")
//                 .unwrap();

//             assert_time_hop(&res_sabr, 2, 3, 2, "SABR");

//             Ok(())
//         })
//     }

//     #[test]
//     fn test_exemple_1() -> Result<(), ASABRError> {
//         for_test_graph(2, |mg, algo_hop: &mut HybridParenting<false, _, _, Hop>| {
//             make_guard!(guard);
//             let mut algo_sabr =
//                 HybridParenting::<false, NoManagement, EVLManager, SABR>::new(guard, mg);

//             let bundle = make_bundle(3, 0, 0.0, 1000);

//             let res_hop = algo_hop
//                 .find_path(mg, 0, mg.node_id_ref(0).unwrap(), &bundle)
//                 .expect("Hop : Routing Failed !")
//                 .unwrap();

//             assert_time_hop(&res_hop, 3, 30, 2, "Hop");

//             let res_sabr = algo_sabr
//                 .find_path(mg, 0, mg.node_id_ref(0).unwrap(), &bundle)
//                 .expect("Hop : Routing Failed !")
//                 .unwrap();

//             assert_time_hop(&res_sabr, 3, 30, 2, "SABR");

//             Ok(())
//         })
//     }

//     #[test]
//     fn test_exemple_2() -> Result<(), ASABRError> {
//         for_test_graph(3, |mg, algo_hop: &mut HybridParenting<false, _, _, Hop>| {
//             make_guard!(guard);
//             let mut algo_sabr =
//                 HybridParenting::<false, NoManagement, EVLManager, SABR>::new(guard, mg);

//             let bundle = make_bundle(4, 0, 0.0, 1000);

//             let res_hop = algo_hop
//                 .find_path(mg, 0, mg.node_id_ref(0).unwrap(), &bundle)
//                 .expect("Hop : Routing Failed !")
//                 .unwrap();

//             assert_time_hop(&res_hop, 4, 50, 3, "Hop");

//             let res_sabr = algo_sabr
//                 .find_path(mg, 0, mg.node_id_ref(0).unwrap(), &bundle)
//                 .expect("Hop : Routing Failed !")
//                 .unwrap();

//             assert_time_hop(&res_sabr, 4, 50, 3, "SABR");

//             Ok(())
//         })
//     }

//     // #[test]
//     // fn test_vnode_anycast_tree() -> Result<(), ASABRError> {
//     //     let mg = vnode_anycast_graph()?;

//     //     let mut algo = HybridParentingTreeExcl::<NoManagement, EVLManager, SABR>::new(mg.clone());

//     //     let bundle = make_bundle(5, 1, 1.0, 2000.0);

//     //     let res = algo
//     //         .get_next(0.0, 0, &bundle, &[][..])
//     //         .expect("Routing to vnode failed!");

//     //     assert!(
//     //         res.by_destination[5].is_some(),
//     //         "VNode V(5) should be reachable"
//     //     );

//     //     let vnode_route = res.by_destination[5].as_ref().unwrap().borrow();
//     //     assert_eq!(
//     //         vnode_route.to_node, 5,
//     //         "Route to_node should be vnode vertex ID (5), got {}",
//     //         vnode_route.to_node
//     //     );

//     //     Ok(())
//     // }

//     // #[test]
//     // fn test_vnode_anycast_path() -> Result<(), ASABRError> {
//     //     let mg = vnode_anycast_graph()?;

//     //     let mut algo = HybridParentingPathExcl::<NoManagement, EVLManager, SABR>::new(mg.clone());

//     //     let bundle = make_bundle(5, 1, 1.0, 2000.0);

//     //     let res = algo
//     //         .get_next(0.0, 0, &bundle, &[][..])
//     //         .expect("Routing to vnode failed!");

//     //     assert!(
//     //         res.by_destination[5].is_some(),
//     //         "VNode V(5) should be reachable via path search"
//     //     );

//     //     let vnode_route = res.by_destination[5].as_ref().unwrap().borrow();
//     //     assert_eq!(
//     //         vnode_route.to_node, 5,
//     //         "Route to_node should be vnode ID (5)"
//     //     );

//     //     Ok(())
//     // }
// }
