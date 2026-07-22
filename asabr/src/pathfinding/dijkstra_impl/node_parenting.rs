extern crate alloc;
use core::{cmp::Ordering, marker::PhantomData};

use alloc::{vec, vec::Vec};

use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    distance::Distance,
    multigraph::{Multigraph, RoutableNodeRef},
    node_manager::NodeManager,
    pathfinding::{
        PathFragment,
        dijkstra::{DijkstraWorkspace, Disktra},
    },
    paths::ViaHop,
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
pub type NodeParenting<'id, D> = Disktra<NodeParentingWorkArea<'id, D>, D>;

/// Not intended for public use, use `NodeParenting` directly
pub struct NodeParentingWorkArea<'id, D> {
    paths: Vec<Option<PathFragment<'id>>>,
    visited: Vec<bool>,
    _phantom: PhantomData<D>,
}

impl<'id, NM: NodeManager, CM: ContactManager, D: Distance<NM, CM>> DijkstraWorkspace<'id, NM, CM>
    for NodeParentingWorkArea<'id, D>
{
    fn new(graph: &Multigraph<'id, NM, CM>) -> Self {
        Self {
            paths: vec![None; graph.get_routable_count()],
            visited: vec![false; graph.get_routable_count()],
            _phantom: PhantomData,
        }
    }

    fn into_pathfinding_output<'a>(self) -> PathFindingOutput<'id, 'a> {
        PathFindingOutput {
            path_tree: crate::parsing::Either::Right(self.paths),
        }
    }

    #[inline(always)]
    fn try_insert(
        &mut self,
        proposition: PathFragment<'id>,
        node: RoutableNodeRef<'id>,
        graph: &Multigraph<'id, NM, CM>,
        bundle: &Bundle,
    ) -> Option<usize> {
        let dest = &mut self.paths[graph.routable_to_usize(node)];
        if dest
            .as_ref()
            .is_none_or(|old| D::cmp(&proposition, old, graph, bundle) == Ordering::Less)
        {
            *dest = Some(proposition);
            Some(graph.routable_to_usize(node))
        } else {
            None
        }
    }
    #[inline(always)]
    fn node_check(&mut self, node: RoutableNodeRef<'id>, graph: &Multigraph<'id, NM, CM>) -> bool {
        !self.visited[graph.routable_to_usize(node)]
    }
    #[inline(always)]
    fn poped_relevant_new(
        &mut self,
        frag: PathFragment<'id>,
        _node: RoutableNodeRef<'id>,
        viaref: usize,
    ) -> (bool, bool, Option<crate::multigraph::INodeRef<'id>>) {
        if self.visited[viaref] {
            (false, false, None)
        } else {
            self.visited[viaref] = true;
            (
                true,
                true,
                frag.via.map(|ViaHop { parent_frag, .. }| unsafe {
                    self.paths[parent_frag]
                        .unwrap()
                        .rx_node
                        .internal()
                        .unwrap_unchecked()
                }),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contact_manager::legacy::evl::EVLManager;
    use crate::contact_plan::asabr_file_lexer::parse_from_iter;
    use crate::distance::hop::Hop;
    use crate::distance::sabr::SABR;
    use crate::multigraph::NodeRef;
    use crate::node_manager::none::NoManagement;
    use crate::pathfinding::ASABRError;
    use crate::pathfinding::test_helpers::*;
    use crate::pathfinding::{Dest, DestAll, Pathfinding};
    use generativity::make_guard;

    #[test]
    fn test_a_to_c_tree() -> Result<(), ASABRError> {
        let graph_str = "node 0 A node 1 B node 2 C
                            contact 0 1 0 2000 100 1
                            contact 1 2 0 2000 100 1";

        let lines = graph_str.lines();
        let contact_plan = parse_from_iter::<NoManagement, EVLManager>(lines).unwrap();
        make_guard!(id);
        let mut graph = Multigraph::new(id, contact_plan).unwrap();

        let ref_0 = match graph.node_id_ref(0.into()).unwrap() {
            NodeRef::I(r) => r,
            _ => panic!(),
        };
        let ref_2 = match graph.node_id_ref(2.into()).unwrap() {
            NodeRef::I(r) => r,
            _ => panic!(),
        };

        // Bundle: Size 100, Tx Duration = 1, Delay = 1
        // Hop 1: Arrival = 0 + 1 + 1 = 2
        // Hop 2: Arrival = 2 + 1 + 1 = 4
        let bundle = make_bundle(2, 100, 2000);

        let mut algo_hop = NodeParenting::<Hop>::new();
        let mut dest = DestAll;

        let res_hop = algo_hop
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest, None)?
            .expect("Hop: Routing Failed!");

        let dest_id_2: usize = ref_2.into();
        let path_hop = res_hop[dest_id_2].as_ref().expect("Path to C should exist");
        assert_eq!(path_hop.recv.end, 4, "Hop: Expected arrival 4");
        assert_eq!(path_hop.hop_count, 2, "Hop: Expected 2 hops");

        let mut algo_sabr = NodeParenting::<SABR>::new();
        let mut dest_sabr = DestAll;

        let res_sabr = algo_sabr
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest_sabr, None)?
            .expect("SABR: Routing Failed!");

        let path_sabr = res_sabr[dest_id_2]
            .as_ref()
            .expect("Path to C should exist");
        assert_eq!(path_sabr.recv.end, 4, "SABR: Expected arrival 4");
        assert_eq!(path_sabr.hop_count, 2, "SABR: Expected 2 hops");

        Ok(())
    }

    #[test]
    fn test_a_to_c_tree_excluded() -> Result<(), ASABRError> {
        let graph_str = "node 0 A node 1 B node 2 C
                            contact 0 1 0 2000 100 1
                            contact 1 2 0 2000 100 1";

        let lines = graph_str.lines();
        let contact_plan = parse_from_iter::<NoManagement, EVLManager>(lines).unwrap();
        make_guard!(id);
        let mut graph = Multigraph::new(id, contact_plan).unwrap();

        let ref_1 = match graph.node_id_ref(1.into()).unwrap() {
            NodeRef::I(r) => r,
            _ => panic!(),
        };
        let real_ref_1: crate::multigraph::RealNodeRef = ref_1.into();
        graph.mark_excluded(&[real_ref_1]);

        let ref_0 = match graph.node_id_ref(0.into()).unwrap() {
            NodeRef::I(r) => r,
            _ => panic!(),
        };

        let bundle = make_bundle(2, 100, 2000);

        let mut algo_hop = NodeParenting::<Hop>::new();
        let mut dest = DestAll;
        let res_hop = algo_hop
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest, None)?
            .expect("Hop: Routing Failed!");

        assert!(res_hop[1].is_none(), "Hop: Node B should be excluded");
        assert!(
            res_hop[2].is_none(),
            "Hop: Node C should not be accessible without B"
        );

        let mut algo_sabr = NodeParenting::<SABR>::new();
        let mut dest_sabr = DestAll;
        let res_sabr = algo_sabr
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest_sabr, None)?
            .expect("SABR: Routing Failed!");

        assert!(res_sabr[1].is_none(), "SABR: Node B should be excluded");
        assert!(
            res_sabr[2].is_none(),
            "SABR: Node C should not be accessible without B"
        );

        Ok(())
    }

    #[test]
    fn test_a_to_c_path_excl() -> Result<(), ASABRError> {
        let graph_str = "node 0 A node 1 B node 2 C
                            contact 0 1 0 2000 100 1
                            contact 1 2 0 2000 100 1";

        let lines = graph_str.lines();
        let contact_plan = parse_from_iter::<NoManagement, EVLManager>(lines).unwrap();
        make_guard!(id);
        let mut graph = Multigraph::new(id, contact_plan).unwrap();

        let ref_1 = match graph.node_id_ref(1.into()).unwrap() {
            NodeRef::I(r) => r,
            _ => panic!(),
        };
        let real_ref_1: crate::multigraph::RealNodeRef = ref_1.into();
        graph.mark_excluded(&[real_ref_1]);

        let ref_0 = match graph.node_id_ref(0.into()).unwrap() {
            NodeRef::I(r) => r,
            _ => panic!(),
        };
        let ref_2 = match graph.node_id_ref(2.into()).unwrap() {
            NodeRef::I(r) => r,
            _ => panic!(),
        };

        let bundle = make_bundle(2, 100, 2000);

        let mut algo_hop = NodeParenting::<Hop>::new();
        let mut dest = Dest::INode(ref_2);

        let res_hop = algo_hop
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest, None)?
            .expect("Hop: Routing Failed!");
        assert!(
            res_hop[2].is_none(),
            "Hop: Node C should not be accessible without B"
        );

        let mut algo_sabr = NodeParenting::<SABR>::new();
        let mut dest_sabr = Dest::INode(ref_2);
        let res_sabr = algo_sabr
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest_sabr, None)?
            .expect("SABR: Routing Failed!");
        assert!(
            res_sabr[2].is_none(),
            "SABR: Node C should not be accessible without B"
        );

        Ok(())
    }

    #[test]
    fn test_two_paths_to_c() -> Result<(), ASABRError> {
        // Paths:
        // A(0) -> C(2) direct (Fewest hops, high delay: 10)
        // A(0) -> B(1) -> C(2) (Fastest time, delay: 1+1=2)
        // A(0) -> D(3) -> C(2) (Alternative path, delay: 3+3=6)
        let graph_str = "node 0 A node 1 B node 2 C node 3 D
                            contact 0 1 0 2000 100 1
                            contact 1 2 0 2000 100 1
                            contact 0 3 0 2000 100 3
                            contact 3 2 0 2000 100 3
                            contact 0 2 0 2000 100 10";

        let lines = graph_str.lines();
        let contact_plan = parse_from_iter::<NoManagement, EVLManager>(lines).unwrap();
        make_guard!(id);
        let mut graph = Multigraph::new(id, contact_plan).unwrap();

        let ref_0 = match graph.node_id_ref(0.into()).unwrap() {
            NodeRef::I(r) => r,
            _ => panic!(),
        };

        // Bundle: Size 100, Tx Duration = 1
        let bundle = make_bundle(2, 100, 2000);

        let mut algo_hop = NodeParenting::<Hop>::new();
        let mut dest_hop = DestAll;
        let res_hop = algo_hop
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest_hop, None)?
            .expect("Hop: Routing Failed!");

        // Hop metric prefers the direct path (1 hop) despite high delay
        let path_hop = res_hop[2].as_ref().unwrap();
        assert_eq!(
            path_hop.recv.end, 11,
            "Hop: Expected arrival 11 via direct path"
        );
        assert_eq!(path_hop.hop_count, 1, "Hop: Expected 1 hop");

        let mut algo_sabr = NodeParenting::<SABR>::new();
        let mut dest_sabr = DestAll;
        let res_sabr = algo_sabr
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest_sabr, None)?
            .expect("SABR: Routing Failed!");

        // SABR metric prefers the fastest time via node B (2 hops)
        let path_sabr = res_sabr[2].as_ref().unwrap();
        assert_eq!(path_sabr.recv.end, 4, "SABR: Expected arrival 4 via node B");
        assert_eq!(path_sabr.hop_count, 2, "SABR: Expected 2 hops");

        Ok(())
    }

    #[test]
    fn test_exemple_1() -> Result<(), ASABRError> {
        let graph_str = "node 0 source node 1 from_C0 node 2 from_C2_C1 node 3 from_C3
                         contact 0 1 0 10 1 0
                         contact 0 2 25 35 1 0
                         contact 1 2 10 20 1 0
                         contact 2 3 30 40 1 0";

        let lines = graph_str.lines();
        let contact_plan = parse_from_iter::<NoManagement, EVLManager>(lines).unwrap();
        make_guard!(id);
        let mut graph = Multigraph::new(id, contact_plan).unwrap();

        let ref_0 = match graph.node_id_ref(0.into()).unwrap() {
            NodeRef::I(r) => r,
            _ => panic!(),
        };
        let ref_3 = match graph.node_id_ref(3.into()).unwrap() {
            NodeRef::I(r) => r,
            _ => panic!(),
        };

        // Bundle: Size 0, Tx Duration = 0
        let bundle = make_bundle(3, 0, 1000);

        let mut algo_hop = NodeParenting::<Hop>::new();
        let mut dest_hop = Dest::INode(ref_3);
        let res_hop = algo_hop
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest_hop, None)?
            .expect("Hop: Routing Failed!");

        let dest_id: usize = ref_3.into();
        let path_hop = res_hop[dest_id].as_ref().unwrap();
        assert_eq!(path_hop.recv.end, 30, "Hop: Expected arrival 30");
        assert_eq!(path_hop.hop_count, 2, "Hop: Expected 2 hops");

        let mut algo_sabr = NodeParenting::<SABR>::new();
        let mut dest_sabr = Dest::INode(ref_3);
        let res_sabr = algo_sabr
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest_sabr, None)?
            .expect("SABR: Routing Failed!");

        let path_sabr = res_sabr[dest_id].as_ref().unwrap();
        assert_eq!(path_sabr.recv.end, 30, "SABR: Expected arrival 30");
        assert_eq!(path_sabr.hop_count, 3, "SABR: Expected 3 hops");

        Ok(())
    }

    #[test]
    fn test_exemple_2() -> Result<(), ASABRError> {
        let graph_str =
            "node 0 source node 1 from_C0 node 2 from_C2_C1 node 3 from_C3 node 4 from_C4
                         contact 0 1 0 10 1 0
                         contact 0 2 25 35 1 0
                         contact 1 2 10 23 1 0
                         contact 2 3 20 40 1 0
                         contact 3 4 50 60 1 0";

        let lines = graph_str.lines();
        let contact_plan = parse_from_iter::<NoManagement, EVLManager>(lines).unwrap();
        make_guard!(id);
        let mut graph = Multigraph::new(id, contact_plan).unwrap();

        let ref_0 = match graph.node_id_ref(0.into()).unwrap() {
            NodeRef::I(r) => r,
            _ => panic!(),
        };
        let ref_4 = match graph.node_id_ref(4.into()).unwrap() {
            NodeRef::I(r) => r,
            _ => panic!(),
        };

        // Bundle: Size 0, Tx Duration = 0
        let bundle = make_bundle(4, 0, 1000);

        let mut algo_hop = NodeParenting::<Hop>::new();
        let mut dest_hop = Dest::INode(ref_4);
        let res_hop = algo_hop
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest_hop, None)?
            .expect("Hop: Routing Failed!");

        let dest_id: usize = ref_4.into();
        let path_hop = res_hop[dest_id].as_ref().unwrap();
        assert_eq!(path_hop.recv.end, 50, "Hop: Expected arrival 50");
        assert_eq!(path_hop.hop_count, 3, "Hop: Expected 3 hops");

        let mut algo_sabr = NodeParenting::<SABR>::new();
        let mut dest_sabr = Dest::INode(ref_4);
        let res_sabr = algo_sabr
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest_sabr, None)?
            .expect("SABR: Routing Failed!");

        let path_sabr = res_sabr[dest_id].as_ref().unwrap();
        assert_eq!(path_sabr.recv.end, 50, "SABR: Expected arrival 50");
        assert_eq!(path_sabr.hop_count, 4, "SABR: Expected 4 hops");

        Ok(())
    }

    #[test]
    fn test_vnode_anycast_tree() -> Result<(), ASABRError> {
        // VNode V(5) points to nodes C(2) and E(4)
        // Path to C(2): A->B->C (arrival = 6)
        // Path to E(4): A->D->E (arrival = 4)
        // The VNode route should target the fastest option: E(4)
        let graph_str = "node 0 A node 1 B node 2 C node 3 D node 4 E
                         vnode 5 V [ 2 , 4 ]
                         contact 0 1 0 2000 100 2
                         contact 1 2 0 2000 100 2
                         contact 0 3 0 2000 100 1
                         contact 3 4 0 2000 100 1";

        let lines = graph_str.lines();
        let contact_plan = parse_from_iter::<NoManagement, EVLManager>(lines).unwrap();
        make_guard!(id);
        let mut graph = Multigraph::new(id, contact_plan).unwrap();

        let ref_0 = match graph.node_id_ref(0.into()).unwrap() {
            NodeRef::I(r) => r,
            _ => panic!(),
        };

        // Bundle: Size 100, Tx Duration = 1
        let bundle = make_bundle(1, 100, 2000);

        let mut algo = NodeParenting::<SABR>::new();
        let mut dest = DestAll;

        let res = algo
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest, None)?
            .expect("Routing to vnode failed!");

        let e_idx: usize = 4;
        assert!(res[e_idx].is_some(), "Real node E(4) should be reachable");
        let path_to_e = res[e_idx].as_ref().unwrap();

        assert_eq!(
            path_to_e.recv.end, 4,
            "Should pick the faster path through E (arrival 4)"
        );

        let c_idx: usize = 2;
        assert!(res[c_idx].is_some(), "Real node C(2) should be reachable");
        let path_to_c = res[c_idx].as_ref().unwrap();

        assert_eq!(path_to_c.recv.end, 6, "Path to C is slower (arrival 6)");

        Ok(())
    }

    /// Tests that routing to a vnode correctly picks the faster path.
    ///
    /// VNode V(5) labels real nodes C(2) and E(4).
    /// The unicast pathfinder should stop at V(5) once it is popped from
    /// the priority queue, having found the best route (through E, faster).
    #[test]
    fn test_vnode_anycast_path() -> Result<(), ASABRError> {
        let graph_str = "node 0 A node 1 B node 2 C node 3 D node 4 E
                         vnode 5 V [ 2 , 4 ]
                         contact 0 1 0 2000 100 2
                         contact 1 2 0 2000 100 2
                         contact 0 3 0 2000 100 1
                         contact 3 4 0 2000 100 1";

        let lines = graph_str.lines();
        let contact_plan = parse_from_iter::<NoManagement, EVLManager>(lines).unwrap();
        make_guard!(id);
        let mut graph = Multigraph::new(id, contact_plan).unwrap();

        let ref_0 = match graph.node_id_ref(0.into()).unwrap() {
            NodeRef::I(r) => r,
            _ => panic!(),
        };
        let ref_5 = match graph.node_id_ref(5.into()).unwrap() {
            NodeRef::V(r) => r,
            _ => panic!(),
        };

        let bundle = make_bundle(1, 100, 2000);

        let mut algo = NodeParenting::<SABR>::new();
        let mut dest = Dest::VNode(ref_5);

        let res = algo
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest, None)?
            .expect("Routing to vnode failed!");

        // Targeted search should resolve to the optimal node within the VNode
        let e_idx: usize = 4;
        assert!(
            res[e_idx].is_some(),
            "Real node E(4) should be the chosen path for the VNode"
        );

        let path_to_e = res[e_idx].as_ref().unwrap();
        assert_eq!(
            path_to_e.recv.end, 4,
            "Should pick the faster path through E even on targeted search"
        );

        Ok(())
    }
}
