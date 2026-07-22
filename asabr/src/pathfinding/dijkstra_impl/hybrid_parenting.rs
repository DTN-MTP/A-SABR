extern crate alloc;
use alloc::{vec, vec::Vec};

use core::{cmp::Ordering, marker::PhantomData};

use super::super::PathFindingOutput;
use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    distance::Distance,
    multigraph::{INodeRef, Multigraph, RoutableNodeRef},
    node_manager::NodeManager,
    pathfinding::{
        dijkstra::{DijkstraWorkspace, Disktra},
        flatten,
    },
    paths::{PathFragment, ViaHop},
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
    ) -> bool;
}

/// Dijkstra pathfinder using the hybrid-parenting work area.
pub type HybridParenting<'id, D, NM, CM> = Disktra<HybridParentingWorkArea<'id, NM, CM, D>, D>;

/// Not intended for public use, use `HybridParenting` directly
pub struct HybridParentingWorkArea<
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
    by_dest_vnode: Vec<Option<usize>>,
    _phantom: PhantomData<fn(NM, CM, D)>,
}

impl<'id, NM: NodeManager, CM: ContactManager, D: Distance<NM, CM> + HybridParentingOrd<NM, CM>>
    DijkstraWorkspace<'id, NM, CM> for HybridParentingWorkArea<'id, NM, CM, D>
{
    fn new(graph: &Multigraph<'id, NM, CM>) -> Self {
        Self {
            possible_paths: Vec::new(),
            by_destination: vec![Vec::new(); graph.get_internal_count()],
            by_dest_vnode: vec![None; graph.get_vnode_count()],
            _phantom: PhantomData,
        }
    }

    fn into_pathfinding_output<'a>(self) -> PathFindingOutput<'id, 'a> {
        flatten(
            &self.possible_paths,
            self.by_destination
                .into_iter()
                .map(|possibilities| possibilities.first().copied())
                .chain(self.by_dest_vnode),
        )
    }

    fn try_insert(
        &mut self,
        proposition: PathFragment<'id>,
        actual_node: RoutableNodeRef<'id>,
        graph: &Multigraph<'id, NM, CM>,
        bundle: &Bundle,
    ) -> Option<usize> {
        match actual_node {
            RoutableNodeRef::I(actual_node) => {
                let new_idx = self.possible_paths.len();
                let routes_for_node = &mut self.by_destination[usize::from(actual_node)];
                if let Some(fst) = routes_for_node.first_mut() {
                    if D::cmp(&proposition, &self.possible_paths[*fst], graph, bundle)
                        == Ordering::Less
                    {
                        if D::keep_both(&proposition, &self.possible_paths[*fst], graph, bundle) {
                            let tmp = *fst;
                            *fst = new_idx;
                            routes_for_node.push(tmp);
                            self.possible_paths.push(proposition);
                            Some(new_idx)
                        } else {
                            self.possible_paths[*fst] = proposition;
                            Some(*fst)
                        }
                    } else {
                        for prop in routes_for_node.iter() {
                            if D::cmp(&proposition, &self.possible_paths[*prop], graph, bundle)
                                == Ordering::Less
                            {
                                self.possible_paths[*prop] = proposition;
                                return Some(*prop);
                            }
                            if !D::keep_both(
                                &proposition,
                                &self.possible_paths[*prop],
                                graph,
                                bundle,
                            ) {
                                return None;
                            }
                        }
                        routes_for_node.push(new_idx);
                        self.possible_paths.push(proposition);
                        Some(new_idx)
                    }
                } else {
                    routes_for_node.push(new_idx);
                    self.possible_paths.push(proposition);
                    Some(new_idx)
                }
            }
            RoutableNodeRef::V(vnode) => match &mut self.by_dest_vnode[usize::from(vnode)] {
                a @ None => {
                    let new_idx = self.possible_paths.len();
                    self.possible_paths.push(proposition);
                    *a = Some(new_idx);
                    Some(new_idx)
                }
                Some(old) => {
                    if D::cmp(&proposition, &self.possible_paths[*old], graph, bundle)
                        == Ordering::Less
                    {
                        self.possible_paths[*old] = proposition;
                        Some(*old)
                    } else {
                        None
                    }
                }
            },
        }
    }
    #[inline(always)]
    fn node_check(
        &mut self,
        _node: RoutableNodeRef<'id>,
        _graph: &Multigraph<'id, NM, CM>,
    ) -> bool {
        true
    }
    #[inline(always)]
    fn poped_relevant_new(
        &mut self,
        frag: PathFragment<'id>,
        node: RoutableNodeRef<'id>,
        viaref: usize,
    ) -> (bool, bool, Option<INodeRef<'id>>) {
        if self.possible_paths[viaref] != frag {
            (false, false, None)
        } else {
            let prev = self.possible_paths[viaref]
                .via
                .map(|ViaHop { parent_frag, .. }| unsafe {
                    self.possible_paths[parent_frag]
                        .rx_node
                        .internal()
                        .unwrap_unchecked()
                });
            match node {
                RoutableNodeRef::I(rnode) => (
                    true,
                    Some(viaref) == self.by_destination[usize::from(rnode)].first().copied(),
                    prev,
                ),
                RoutableNodeRef::V(_vnode) => (true, true, prev),
            }
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

        let bundle = make_bundle(2, 100, 2000);

        let mut algo_hop = HybridParenting::<Hop, NoManagement, EVLManager>::new();
        let mut dest = DestAll;
        let res_hop = algo_hop
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest, None)?
            .expect("Hop: Routing Failed!");

        let dest_id_2: usize = ref_2.into();
        let path_hop = res_hop[dest_id_2].as_ref().unwrap();
        assert_eq!(path_hop.recv.end, 4, "Hop: Expected arrival 4");
        assert_eq!(path_hop.hop_count, 2, "Hop: Expected 2 hops");

        let mut algo_sabr = HybridParenting::<SABR, NoManagement, EVLManager>::new();
        let mut dest_sabr = DestAll;
        let res_sabr = algo_sabr
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest_sabr, None)?
            .expect("SABR: Routing Failed!");

        let path_sabr = res_sabr[dest_id_2].as_ref().unwrap();
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

        let mut algo_hop = HybridParenting::<Hop, NoManagement, EVLManager>::new();
        let mut dest = DestAll;
        let res_hop = algo_hop
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest, None)?
            .expect("Hop: Routing Failed!");

        assert!(res_hop[1].is_none(), "Hop: Node B should be excluded");
        assert!(
            res_hop[2].is_none(),
            "Hop: Node C should not be accessible without B"
        );

        let mut algo_sabr = HybridParenting::<SABR, NoManagement, EVLManager>::new();
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

        let mut algo_hop = HybridParenting::<Hop, NoManagement, EVLManager>::new();
        let mut dest = Dest::INode(ref_2);
        let res_hop = algo_hop
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest, None)?
            .expect("Hop: Routing Failed!");
        assert!(
            res_hop[2].is_none(),
            "Hop: Node C should not be accessible without B"
        );

        let mut algo_sabr = HybridParenting::<SABR, NoManagement, EVLManager>::new();
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

        let bundle = make_bundle(2, 100, 2000);

        let mut algo_hop = HybridParenting::<Hop, NoManagement, EVLManager>::new();
        let mut dest_hop = DestAll;
        let res_hop = algo_hop
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest_hop, None)?
            .expect("Hop: Routing Failed!");

        let path_hop = res_hop[2].as_ref().unwrap();
        assert_eq!(
            path_hop.recv.end, 11,
            "Hop: Expected arrival 11 via direct path"
        );
        assert_eq!(path_hop.hop_count, 1, "Hop: Expected 1 hop");

        let mut algo_sabr = HybridParenting::<SABR, NoManagement, EVLManager>::new();
        let mut dest_sabr = DestAll;
        let res_sabr = algo_sabr
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest_sabr, None)?
            .expect("SABR: Routing Failed!");

        let path_sabr = res_sabr[2].as_ref().unwrap();
        assert_eq!(path_sabr.recv.end, 4, "SABR: Expected arrival 4 via B");
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

        let bundle = make_bundle(3, 0, 1000);

        let mut algo_hop = HybridParenting::<Hop, NoManagement, EVLManager>::new();
        let mut dest_hop = Dest::INode(ref_3);
        let res_hop = algo_hop
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest_hop, None)?
            .expect("Hop: Routing Failed!");

        let dest_id: usize = ref_3.into();
        let path_hop = res_hop[dest_id].as_ref().unwrap();
        assert_eq!(path_hop.recv.end, 30, "Hop: Expected arrival 30");
        assert_eq!(path_hop.hop_count, 2, "Hop: Expected 2 hops");

        let mut algo_sabr = HybridParenting::<SABR, NoManagement, EVLManager>::new();
        let mut dest_sabr = Dest::INode(ref_3);
        let res_sabr = algo_sabr
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest_sabr, None)?
            .expect("SABR: Routing Failed!");

        let path_sabr = res_sabr[dest_id].as_ref().unwrap();
        assert_eq!(path_sabr.recv.end, 30, "SABR: Expected arrival 30");
        assert_eq!(path_sabr.hop_count, 2, "SABR: Expected 2 hops");

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

        let bundle = make_bundle(4, 0, 1000);

        let mut algo_hop = HybridParenting::<Hop, NoManagement, EVLManager>::new();
        let mut dest_hop = Dest::INode(ref_4);
        let res_hop = algo_hop
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest_hop, None)?
            .expect("Hop: Routing Failed!");

        let dest_id: usize = ref_4.into();
        let path_hop = res_hop[dest_id].as_ref().unwrap();
        assert_eq!(path_hop.recv.end, 50, "Hop: Expected arrival 50");
        assert_eq!(path_hop.hop_count, 3, "Hop: Expected 3 hops");

        let mut algo_sabr = HybridParenting::<SABR, NoManagement, EVLManager>::new();
        let mut dest_sabr = Dest::INode(ref_4);
        let res_sabr = algo_sabr
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest_sabr, None)?
            .expect("SABR: Routing Failed!");

        let path_sabr = res_sabr[dest_id].as_ref().unwrap();
        assert_eq!(path_sabr.recv.end, 50, "SABR: Expected arrival 50");
        assert_eq!(path_sabr.hop_count, 3, "SABR: Expected 3 hops");

        Ok(())
    }

    #[test]
    fn test_vnode_anycast_tree() -> Result<(), ASABRError> {
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

        let bundle = make_bundle(1, 100, 2000);

        let mut algo = HybridParenting::<SABR, NoManagement, EVLManager>::new();
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

        let mut algo = HybridParenting::<SABR, NoManagement, EVLManager>::new();
        let mut dest = Dest::VNode(ref_5);

        let res = algo
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest, None)?
            .expect("Routing to vnode failed!");

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
