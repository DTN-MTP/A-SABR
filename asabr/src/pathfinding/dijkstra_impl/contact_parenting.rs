extern crate alloc;
use alloc::{
    collections::btree_map::{BTreeMap, Entry},
    vec,
    vec::Vec,
};
use core::{cmp::Ordering, marker::PhantomData};

use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    distance::Distance,
    multigraph::{ContactRef, INodeRef, Multigraph, RoutableNodeRef},
    node_manager::NodeManager,
    pathfinding::{
        dijkstra::{DijkstraWorkspace, Disktra},
        flatten,
    },
    paths::{PathFragment, ViaHop},
};

use super::super::PathFindingOutput;

/// A contact parenting (contact graph) implementation of Dijkstra algorithm.
///
/// This implementation includes shortest-path tree construction.
///
/// # Type Parameters
///
/// * `NM` - A type that implements the `NodeManager` trait.
/// * `CM` - A type that implements the `ContactManager` trait.
pub type ContactParenting<'id, NM, CM, D> = Disktra<ContactParentingWorkArea<'id, NM, CM, D>, D>;

/// Not intended for public use, use `ContactParenting` directly
pub struct ContactParentingWorkArea<'id, NM: NodeManager, CM: ContactManager, D: Distance<NM, CM>> {
    /// A vector storing all keeped path to a node without sorting for easy reference.
    possible_paths: Vec<PathFragment<'id>>,
    /// A vector containing (option of index of) pathfragment, to reach a given destination.
    by_destination: Vec<Option<usize>>,
    by_dest_vnode: Vec<Option<usize>>,
    /// Visited contacts, grouped by node.
    visited: Vec<BTreeMap<ContactRef<'id>, usize>>,
    _phantom: PhantomData<fn(NM, CM, D)>,
}

impl<'id, NM: NodeManager, CM: ContactManager, D: Distance<NM, CM>> DijkstraWorkspace<'id, NM, CM>
    for ContactParentingWorkArea<'id, NM, CM, D>
{
    /// Constructs a new `ContactParenting` instance with the provided nodes and contacts.
    fn new(graph: &Multigraph<'id, NM, CM>) -> Self {
        Self {
            possible_paths: Vec::new(),
            by_destination: vec![None; graph.get_internal_count()],
            by_dest_vnode: vec![None; graph.get_vnode_count()],
            visited: vec![BTreeMap::new(); graph.get_nonvirtualnode_count()],
            _phantom: PhantomData,
        }
    }
    fn into_pathfinding_output<'a>(self) -> PathFindingOutput<'id, 'a> {
        flatten(
            &self.possible_paths,
            self.by_destination.into_iter().chain(self.by_dest_vnode),
        )
    }

    fn try_insert(
        &mut self,
        proposition: PathFragment<'id>,
        node: RoutableNodeRef<'id>,
        graph: &Multigraph<'id, NM, CM>,
        bundle: &Bundle,
    ) -> Option<usize> {
        // println!("prop for {node}: {proposition}");
        let new_idx = self.possible_paths.len();
        let result;

        match proposition.via {
            None => {
                self.possible_paths.push(proposition);
                result = new_idx;
            }
            Some(ViaHop { contact, .. }) => {
                match self.visited[usize::from(graph.into_nodeid(proposition.rx_node.into()))]
                    .entry(contact)
                {
                    Entry::Vacant(vacant_entry) => {
                        vacant_entry.insert(new_idx);
                        self.possible_paths.push(proposition);
                        result = new_idx;
                    }
                    Entry::Occupied(occupied_entry) => {
                        let old = *occupied_entry.get();
                        if D::cmp(&proposition, &self.possible_paths[old], graph, bundle)
                            == Ordering::Less
                        {
                            result = old;
                            self.possible_paths[old] = proposition
                        } else {
                            return None;
                        }
                    }
                }
            }
        }
        let for_node = match node {
            RoutableNodeRef::I(inode) => &mut self.by_destination[usize::from(inode)],
            RoutableNodeRef::V(vnode) => &mut self.by_dest_vnode[usize::from(vnode)],
        };
        match for_node {
            Some(for_node) => {
                if D::cmp(&proposition, &self.possible_paths[*for_node], graph, bundle)
                    == Ordering::Less
                {
                    *for_node = result;
                }
            }
            None => {
                *for_node = Some(result);
            }
        }
        Some(result)
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
        if self.possible_paths[viaref] == frag {
            let prev = frag.via.map(|ViaHop { parent_frag, .. }| unsafe {
                self.possible_paths[parent_frag]
                    .rx_node
                    .internal()
                    .unwrap_unchecked()
            });
            match node {
                RoutableNodeRef::I(inode) => (
                    true,
                    self.by_destination[usize::from(inode)] == Some(viaref),
                    prev,
                ),
                RoutableNodeRef::V(vnode) => (
                    true,
                    self.by_dest_vnode[usize::from(vnode)] == Some(viaref),
                    prev,
                ),
            }
        } else {
            (false, false, None)
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

        let mut algo_hop = ContactParenting::<NoManagement, EVLManager, Hop>::new();
        let mut dest = DestAll;
        let res_hop = algo_hop
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest, None)?
            .expect("Hop: Routing Failed!");

        let dest_id_2: usize = ref_2.into();
        let path_hop = res_hop[dest_id_2].as_ref().unwrap();
        assert_eq!(path_hop.recv.end, 4, "Hop: Expected arrival 4");
        assert_eq!(path_hop.hop_count, 2, "Hop: Expected 2 hops");

        let mut algo_sabr = ContactParenting::<NoManagement, EVLManager, SABR>::new();
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

        let mut algo_hop = ContactParenting::<NoManagement, EVLManager, Hop>::new();
        let mut dest = DestAll;
        let res_hop = algo_hop
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest, None)?
            .expect("Hop: Routing Failed!");

        assert!(res_hop[1].is_none(), "Hop: Node B should be excluded");
        assert!(
            res_hop[2].is_none(),
            "Hop: Node C should not be accessible without B"
        );

        let mut algo_sabr = ContactParenting::<NoManagement, EVLManager, SABR>::new();
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

        let mut algo_hop = ContactParenting::<NoManagement, EVLManager, Hop>::new();
        let mut dest = Dest::INode(ref_2);
        let res_hop = algo_hop
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest, None)?
            .expect("Hop: Routing Failed!");
        assert!(
            res_hop[2].is_none(),
            "Hop: Node C should not be accessible without B"
        );

        let mut algo_sabr = ContactParenting::<NoManagement, EVLManager, SABR>::new();
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

        let mut algo_hop = ContactParenting::<NoManagement, EVLManager, Hop>::new();
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

        let mut algo_sabr = ContactParenting::<NoManagement, EVLManager, SABR>::new();
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

        let mut algo_hop = ContactParenting::<NoManagement, EVLManager, Hop>::new();
        let mut dest_hop = Dest::INode(ref_3);
        let res_hop = algo_hop
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest_hop, None)?
            .expect("Hop: Routing Failed!");

        let dest_id: usize = ref_3.into();
        let path_hop = res_hop[dest_id].as_ref().unwrap();
        assert_eq!(path_hop.recv.end, 30, "Hop: Expected arrival 30");
        assert_eq!(path_hop.hop_count, 2, "Hop: Expected 2 hops");

        let mut algo_sabr = ContactParenting::<NoManagement, EVLManager, SABR>::new();
        let mut dest_sabr = Dest::INode(ref_3);
        let res_sabr = algo_sabr
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest_sabr, None)?
            .expect("SABR: Routing Failed!");

        let path_sabr = res_sabr[dest_id].as_ref().unwrap();
        assert_eq!(path_sabr.recv.end, 30, "SABR: Expected arrival 30");
        assert_eq!(
            path_sabr.hop_count, 2,
            "SABR: Expected 2 hops for contact graph tie-break"
        );

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

        let mut algo_hop = ContactParenting::<NoManagement, EVLManager, Hop>::new();
        let mut dest_hop = Dest::INode(ref_4);
        let res_hop = algo_hop
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest_hop, None)?
            .expect("Hop: Routing Failed!");

        let dest_id: usize = ref_4.into();
        let path_hop = res_hop[dest_id].as_ref().unwrap();
        assert_eq!(path_hop.recv.end, 50, "Hop: Expected arrival 50");
        assert_eq!(path_hop.hop_count, 3, "Hop: Expected 3 hops");

        let mut algo_sabr = ContactParenting::<NoManagement, EVLManager, SABR>::new();
        let mut dest_sabr = Dest::INode(ref_4);
        let res_sabr = algo_sabr
            .find_path(&mut graph, 0, ref_0, &bundle, &mut dest_sabr, None)?
            .expect("SABR: Routing Failed!");

        let path_sabr = res_sabr[dest_id].as_ref().unwrap();
        assert_eq!(path_sabr.recv.end, 50, "SABR: Expected arrival 50");
        assert_eq!(path_sabr.hop_count, 4, "SABR: Expected 4 hops");

        Ok(())
    }
}
