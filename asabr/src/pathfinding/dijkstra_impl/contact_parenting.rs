extern crate alloc;
use alloc::{collections::BTreeSet, vec, vec::Vec};
use core::{cmp::Ordering, marker::PhantomData};

use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    distance::Distance,
    multigraph::{ContactRef, Multigraph, NodeRef, RNodeRef},
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
    visited: Vec<BTreeSet<ContactRef<'id>>>,
    _phantom: PhantomData<fn(NM, CM, D)>,
}

impl<'id, NM: NodeManager, CM: ContactManager, D: Distance<NM, CM>> DijkstraWorkspace<'id, NM, CM>
    for ContactParentingWorkArea<'id, NM, CM, D>
{
    /// Constructs a new `ContactParenting` instance with the provided nodes and contacts.
    #[inline(always)]
    fn new(graph: &Multigraph<'id, NM, CM>) -> Self {
        Self {
            possible_paths: Vec::new(),
            by_destination: vec![None; graph.get_rnode_count()],
            by_dest_vnode: vec![None; graph.get_vnode_count()],
            visited: vec![BTreeSet::new(); graph.get_nonvirtualnode_count()],
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
        node: NodeRef<'id>,
        graph: &Multigraph<'id, NM, CM>,
        bundle: &Bundle,
    ) -> Option<usize> {
        let new_idx = self.possible_paths.len();
        let route_for_node = match node {
            NodeRef::R(rnode) => &mut self.by_destination[usize::from(rnode)],
            NodeRef::V(vnode) => &mut self.by_dest_vnode[usize::from(vnode)],
        };

        if proposition.via.is_none_or(|ViaHop { contact, .. }| {
            self.visited[usize::from(proposition.rx_node)].insert(contact)
        }) {
            match route_for_node {
                None => {
                    self.possible_paths.push(proposition);
                    *route_for_node = Some(new_idx);
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
            }
        } else {
            None
        }
    }

    fn node_check(&mut self, _node: NodeRef<'id>, _graph: &Multigraph<'id, NM, CM>) -> bool {
        true
    }
    fn poped_relevant_new(
        &mut self,
        frag: PathFragment<'id>,
        node: NodeRef<'id>,
        viaref: usize,
    ) -> (bool, bool, Option<RNodeRef<'id>>) {
        if self.possible_paths[viaref] == frag {
            let prev = frag
                .via
                .map(|ViaHop { parent_frag, .. }| self.possible_paths[parent_frag].rx_node);
            match node {
                NodeRef::R(rnode) => (
                    true,
                    self.by_destination[usize::from(rnode)] == Some(viaref),
                    prev,
                ),
                NodeRef::V(_vnode) => (true, true, prev),
            }
        } else {
            (false, false, None)
        }
    }
}

// TODO: Remake test based on hybrid parenting ones

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

//         let mut algo_hop =
//             ContactParentingTreeExcl::<NoManagement, EVLManager, Hop>::new(mg.clone());
//         let mut algo_sabr =
//             ContactParentingTreeExcl::<NoManagement, EVLManager, SABR>::new(mg.clone());

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

//         let mut algo_hop =
//             ContactParentingTreeExcl::<NoManagement, EVLManager, Hop>::new(mg.clone());
//         let mut algo_sabr =
//             ContactParentingTreeExcl::<NoManagement, EVLManager, SABR>::new(mg.clone());

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

//         let mut algo_hop =
//             ContactParentingPathExcl::<NoManagement, EVLManager, Hop>::new(mg.clone());
//         let mut algo_sabr =
//             ContactParentingPathExcl::<NoManagement, EVLManager, SABR>::new(mg.clone());

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

//         let mut algo_hop =
//             ContactParentingTreeExcl::<NoManagement, EVLManager, Hop>::new(mg.clone());
//         let mut algo_sabr =
//             ContactParentingTreeExcl::<NoManagement, EVLManager, SABR>::new(mg.clone());

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

//         let mut algo_hop = ContactParentingPath::<NoManagement, EVLManager, Hop>::new(mg.clone());
//         let mut algo_sabr = ContactParentingPath::<NoManagement, EVLManager, SABR>::new(mg.clone());

//         let bundle = make_bundle(3, 0, 0.0, 1000.0);

//         let res_hop = algo_hop
//             .get_next(0.0, 0, &bundle, &[][..])
//             .expect("Routing Failed !");

//         assert_time_hop(&res_hop, 3, 30.0, 2, "Hop");

//         let res_sabr = algo_sabr
//             .get_next(0.0, 0, &bundle, &[][..])
//             .expect("Routing Failed !");

//         assert_time_hop(&res_sabr, 3, 30.0, 2, "SABR");

//         Ok(())
//     }

//     #[test]
//     fn test_exemple_2() -> Result<(), ASABRError> {
//         let mg = exemple_2_graph()?;

//         let mut algo_hop = ContactParentingPath::<NoManagement, EVLManager, Hop>::new(mg.clone());
//         let mut algo_sabr = ContactParentingPath::<NoManagement, EVLManager, SABR>::new(mg.clone());

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
// }
