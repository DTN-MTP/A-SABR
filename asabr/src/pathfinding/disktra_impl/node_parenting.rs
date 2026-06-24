extern crate alloc;
use alloc::{vec, vec::Vec};

use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    multigraph::Multigraph,
    node_manager::NodeManager,
    pathfinding::{
        PathFragment,
        disktra::{Disktra, DisktraWorkspace},
    },
    types::NodeID,
};

use super::super::PathFindingOutput;

/// A node parenting (node graph, SPSN v1) implementation of Dijkstra algorithm.
///
/// Use this implementation for optimized resource utilization.
///
/// # Type Parameters
/// * `TREE` wether to calculate a full path tree or stop upon reaching the bundle first destination
/// * `NM` - A type that implements the `NodeManager` trait.
/// * `CM` - A type that implements the `ContactManager` trait.
/// * `D` - A type that implements the `Distance<NM, CM>` trait.
pub type NodeParenting<'id, D> = Disktra<NodeParentingWorkArea<'id>, D>;

pub struct NodeParentingWorkArea<'id> {
    paths: Vec<Option<PathFragment<'id>>>,
}

impl<'id, NM: NodeManager, CM: ContactManager> DisktraWorkspace<'id, NM, CM>
    for NodeParentingWorkArea<'id>
{
    fn new(graph: &Multigraph<'id, NM, CM>) -> Self {
        Self {
            paths: vec![None; graph.get_rnode_count()],
        }
    }

    fn into_pathfinding_output(self) -> PathFindingOutput<'id, 'static> {
        PathFindingOutput {
            path_tree: crate::parsing::Either::Right(self.paths),
        }
    }

    #[inline(always)]
    fn try_insert(
        &mut self,
        proposition: PathFragment<'id>,
        actual_node: crate::multigraph::RNodeRef<'id>,
        graph: &Multigraph<'id, NM, CM>,
        bundle: &Bundle,
    ) -> (Option<usize>, bool) {
        let dest = &mut self.paths[NodeID::from(actual_node) as usize];
        if dest.is_some() {
            (None, false)
        } else {
            *dest = Some(proposition);
            (Some(NodeID::from(actual_node) as usize), true)
        }
    }
    #[inline(always)]
    fn node_check(&mut self, node: crate::multigraph::RNodeRef<'id>) -> bool {
        self.paths[NodeID::from(node) as usize].is_none()
    }

    #[inline(always)]
    fn fragment_check(
        &mut self,
        proposition: PathFragment<'id>,
        dest_node: crate::multigraph::RNodeRef<'id>,
        graph: &Multigraph<'id, NM, CM>,
        bundle: &Bundle,
    ) -> bool {
        true
    }
}

//TODO: Restore tests by interpolation between these and hybrid parenting ones

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::contact_manager::legacy::evl::EVLManager;
//     use crate::distance::hop::Hop;
//     use crate::distance::sabr::SABR;
//     use crate::node_manager::none::NoManagement;
//     use crate::pathfinding::ASABRError;
//     use crate::pathfinding::test_helpers::*;

//     #[test]
//     fn test_a_to_c_tree() -> Result<(), ASABRError> {
//         let mg = unit_graph_test()?;

//         let mut algo_hop = NodeParentingTreeExcl::<NoManagement, EVLManager, Hop>::new(mg.clone());
//         let mut algo_sabr =
//             NodeParentingTreeExcl::<NoManagement, EVLManager, SABR>::new(mg.clone());

//         let bundle = make_bundle(2, 1, 1.0, 2000.0);

//         let res_hop = algo_hop
//             .get_next(0.0, 0, &bundle, &[][..])
//             .expect("Hop : Routing Failed !");

//         assert_time_hop(&res_hop, 2, 2.02, 2, "Hop");

//         let res_sabr = algo_sabr
//             .get_next(0.0, 0, &bundle, &[][..])
//             .expect("SABR : Routing Failed !");

//         assert_time_hop(&res_sabr, 2, 2.02, 2, "SABR");

//         Ok(())
//     }

//     #[test]
//     fn test_a_to_c_tree_excluded() -> Result<(), ASABRError> {
//         let mg = unit_graph_test()?;

//         let mut algo_hop = NodeParentingTreeExcl::<NoManagement, EVLManager, Hop>::new(mg.clone());
//         let mut algo_sabr =
//             NodeParentingTreeExcl::<NoManagement, EVLManager, SABR>::new(mg.clone());

//         let bundle = make_bundle(2, 1, 1.0, 2000.0);
//         let excluded = [1];

//         let res_hop = algo_hop
//             .get_next(0.0, 0, &bundle, &excluded[..])
//             .expect("Hop : Routing Failed !");
//         assert!(
//             res_hop.by_destination[1].is_none(),
//             "Hop : B should be excluded"
//         );
//         assert!(
//             res_hop.by_destination[2].is_none(),
//             "Hop : C should not be accessible without B"
//         );

//         let res_sabr = algo_sabr
//             .get_next(0.0, 0, &bundle, &excluded[..])
//             .expect("SABR : Routing Failed !");
//         assert!(
//             res_sabr.by_destination[1].is_none(),
//             "SABR : B should be excluded"
//         );
//         assert!(
//             res_sabr.by_destination[2].is_none(),
//             "SABR : C should not be accessible without B"
//         );

//         Ok(())
//     }

//     #[test]
//     fn test_a_to_c_path_excl() -> Result<(), ASABRError> {
//         let mg = unit_graph_test()?;

//         let mut algo_hop = NodeParentingPathExcl::<NoManagement, EVLManager, Hop>::new(mg.clone());
//         let mut algo_sabr =
//             NodeParentingPathExcl::<NoManagement, EVLManager, SABR>::new(mg.clone());

//         let bundle = make_bundle(2, 1, 1.0, 2000.0);
//         let excluded = [1];

//         let res_hop = algo_hop
//             .get_next(0.0, 0, &bundle, &excluded[..])
//             .expect("Hop : Routing Failed !");
//         assert!(
//             res_hop.by_destination[2].is_none(),
//             "Hop : C should not be accessible without B"
//         );

//         let res_sabr = algo_sabr
//             .get_next(0.0, 0, &bundle, &excluded[..])
//             .expect("SABR : Routing Failed !");
//         assert!(
//             res_sabr.by_destination[2].is_none(),
//             "SABR : C should not be accessible without B"
//         );

//         Ok(())
//     }

//     #[test]
//     fn test_two_paths_to_c() -> Result<(), ASABRError> {
//         let mg = five_contact_graph_test()?;

//         let mut algo_hop = NodeParentingTreeExcl::<NoManagement, EVLManager, Hop>::new(mg.clone());
//         let mut algo_sabr =
//             NodeParentingTreeExcl::<NoManagement, EVLManager, SABR>::new(mg.clone());

//         let bundle = make_bundle(2, 1, 1.0, 2000.0);

//         let res_hop = algo_hop
//             .get_next(0.0, 0, &bundle, &[][..])
//             .expect("Hop : Routing Failed !");

//         assert_time_hop(&res_hop, 2, 10.01, 1, "Hop");

//         let res_sabr = algo_sabr
//             .get_next(0.0, 0, &bundle, &[][..])
//             .expect("SABR : Routing Failed !");

//         assert_time_hop(&res_sabr, 2, 0.13, 2, "SABR");

//         Ok(())
//     }

//     #[test]
//     fn test_exemple_1() -> Result<(), ASABRError> {
//         let mg = exemple_1_graph()?;

//         let mut algo_hop = NodeParentingPath::<NoManagement, EVLManager, Hop>::new(mg.clone());
//         let mut algo_sabr = NodeParentingPath::<NoManagement, EVLManager, SABR>::new(mg.clone());

//         let bundle = make_bundle(3, 0, 0.0, 1000.0);

//         let res_hop = algo_hop
//             .get_next(0.0, 0, &bundle, &[][..])
//             .expect("Routing Failed !");

//         assert_time_hop(&res_hop, 3, 30.0, 2, "Hop");

//         let res_sabr = algo_sabr
//             .get_next(0.0, 0, &bundle, &[][..])
//             .expect("Routing Failed !");

//         assert_time_hop(&res_sabr, 3, 30.0, 3, "SABR");

//         Ok(())
//     }

//     #[test]
//     fn test_exemple_2() -> Result<(), ASABRError> {
//         let mg = exemple_2_graph()?;

//         let mut algo_hop = NodeParentingPath::<NoManagement, EVLManager, Hop>::new(mg.clone());
//         let mut algo_sabr = NodeParentingPath::<NoManagement, EVLManager, SABR>::new(mg.clone());

//         let bundle = make_bundle(4, 0, 0.0, 1000.0);

//         let res_hop = algo_hop
//             .get_next(0.0, 0, &bundle, &[][..])
//             .expect("Routing Failed !");

//         assert_time_hop(&res_hop, 4, 50.0, 3, "Hop");

//         let res_sabr = algo_sabr
//             .get_next(0.0, 0, &bundle, &[][..])
//             .expect("Routing Failed !");

//         assert_time_hop(&res_sabr, 4, 50.0, 4, "SABR");

//         Ok(())
//     }

//     /// Tests anycast routing via a vnode.
//     ///
//     /// VNode V(5) labels real nodes C(2) and E(4).
//     /// Path to C: A->B->C (arrival = 2.02, 2 hops)
//     /// Path to E: A->D->E (arrival = 1.01, 2 hops)
//     ///
//     /// Routing to vnode V(5) should find the faster path through E.
//     #[test]
//     fn test_vnode_anycast_tree() -> Result<(), ASABRError> {
//         let mg = vnode_anycast_graph()?;

//         let mut algo = NodeParentingTreeExcl::<NoManagement, EVLManager, SABR>::new(mg.clone());

//         // Destination is the vnode V(5)
//         let bundle = make_bundle(5, 1, 1.0, 2000.0);

//         let res = algo
//             .get_next(0.0, 0, &bundle, &[][..])
//             .expect("Routing to vnode failed!");

//         // The vnode vertex (index 5) should have a route
//         assert!(
//             res.by_destination[5].is_some(),
//             "VNode V(5) should be reachable"
//         );

//         let vnode_route = res.by_destination[5].as_ref().unwrap().borrow();
//         // to_node should be the vnode vertex ID (5), not a real node ID
//         assert_eq!(
//             vnode_route.to_node, 5,
//             "Route to vnode should have to_node = vnode vertex ID (5), got {}",
//             vnode_route.to_node
//         );

//         // The real nodes C(2) and E(4) should also be reachable
//         assert!(
//             res.by_destination[2].is_some(),
//             "Real node C(2) should be reachable"
//         );
//         assert!(
//             res.by_destination[4].is_some(),
//             "Real node E(4) should be reachable"
//         );

//         // The vnode route's ViaHop should reference real nodes (not the vnode)
//         let via = vnode_route
//             .via
//             .as_ref()
//             .expect("VNode route should have a ViaHop");
//         let real_rx_id = via.rx_node.borrow().info.id;
//         assert!(
//             real_rx_id == 2 || real_rx_id == 4,
//             "ViaHop rx_node should be a real node in the vnode group (2 or 4), got {real_rx_id}",
//         );

//         Ok(())
//     }

//     /// Tests that routing to a vnode correctly picks the faster path.
//     ///
//     /// VNode V(5) labels real nodes C(2) and E(4).
//     /// The unicast pathfinder should stop at V(5) once it is popped from
//     /// the priority queue, having found the best route (through E, faster).
//     #[test]
//     fn test_vnode_anycast_path() -> Result<(), ASABRError> {
//         let mg = vnode_anycast_graph()?;

//         let mut algo = NodeParentingPathExcl::<NoManagement, EVLManager, SABR>::new(mg.clone());

//         // Destination is the vnode V(5)
//         let bundle = make_bundle(5, 1, 1.0, 2000.0);

//         let res = algo
//             .get_next(0.0, 0, &bundle, &[][..])
//             .expect("Routing to vnode failed!");

//         // V(5) should be reachable
//         assert!(
//             res.by_destination[5].is_some(),
//             "VNode V(5) should be reachable via path search"
//         );

//         let vnode_route = res.by_destination[5].as_ref().unwrap().borrow();
//         assert_eq!(
//             vnode_route.to_node, 5,
//             "Route to_node should be vnode ID (5)"
//         );

//         Ok(())
//     }
// }
