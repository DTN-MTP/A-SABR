extern crate alloc;
use alloc::{collections::BTreeSet, vec, vec::Vec};
use core::marker::PhantomData;
use generativity::Guard;

use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    distance::{Distance, prio_queue::PrioQueue},
    multigraph::Multigraph,
    node_manager::NodeManager,
    parsing::Either,
    paths::PathFragment,
    types::{Date, NodeID},
};

use super::{PathFindingOutput, Pathfinding, try_make_hop};

/// A contact parenting (contact graph) implementation of Dijkstra algorithm.
///
/// This implementation includes shortest-path tree construction.
///
/// # Type Parameters
///
/// * `NM` - A type that implements the `NodeManager` trait.
/// * `CM` - A type that implements the `ContactManager` trait.
pub struct ContactParenting<
    'id,
    const tree_output: bool,
    NM: NodeManager,
    CM: ContactManager,
    D: Distance<NM, CM>,
> {
    // /// For tree construction, tracks the nodes visited as transmitters.
    // visited_as_tx_ids: Vec<bool>,
    // /// For tree construction, tracks the nodes visited as receivers.
    // visited_as_rx_ids: Vec<bool>,
    // /// For tree construction, tracks the count of nodes visited as transmitters.
    // visited_as_tx_count: usize,
    // /// For tree construction, tracks the count of nodes visited as receivers.
    // visited_as_rx_count: usize,
    _phantom: PhantomData<fn(&'id (), D, NM, CM)>,
}

impl<'id, const tree_output: bool, NM: NodeManager, CM: ContactManager, D: Distance<NM, CM>>
    Pathfinding<'id, NM, CM> for ContactParenting<'id, tree_output, NM, CM, D>
{
    /// Constructs a new `ContactParenting` instance with the provided nodes and contacts.
    fn new(_guard: Guard, _multigraph: &Multigraph<'id, NM, CM>) -> Self {
        Self {
            // visited_as_tx_ids: vec![false; node_count],
            // visited_as_rx_ids: vec![false; node_count],
            // visited_as_tx_count: 1,
            // visited_as_rx_count: 1,
            _phantom: PhantomData,
        }
    }

    // / Finds the next route based on the current state and available contacts.
    // /
    // / This method uses a priority queue to explore potential routes from a source node,
    // / considering the current time, bundle, and nodes to exclude from the pathfinding.
    // /
    // / # Parameters
    // /
    // / * `current_time` - The current time used for evaluating routes.
    // / * `source` - The `NodeID` of the source node from which to begin pathfinding.
    // / * `bundle` - The `Bundle` associated with the pathfinding operation.
    // / * `excluded_nodes_sorted` - A sorted list of `NodeID`s to be excluded from the pathfinding.
    // /
    // / # Returns
    // /
    // / * `<ResultPathFindingOutput<NM, CM>, ASABRError>` - The resulting pathfinding output, including the routes found,
    // /   or an error if the operation fails.
    fn find_path<'a>(
        &'a mut self,
        multigraph: &mut Multigraph<'id, NM, CM>,
        current_time: Date,
        source: crate::multigraph::NodeRef<'id>,
        bundle: &Bundle,
    ) -> Result<Option<PathFindingOutput<'id, 'a>>, crate::errors::ASABRError> {
        let mut by_dest = vec![None; multigraph.get_rnode_count()];
        let mut visited = vec![BTreeSet::new(); multigraph.get_rnode_count()];
        let mut all = Vec::with_capacity(multigraph.get_rnode_count());

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
            by_dest[bundle.destinations[0] as usize].is_none()
        }) && let Some(((path, node), ())) = prioqueue.pop_min(multigraph, bundle)
        {
            if tree_output && by_dest[NodeID::from(node) as usize].is_none() {
                reached += 1;
            }
            let viaref = all.len();

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
                    if tree_output && by_dest[NodeID::from(neighbor) as usize].is_none() {
                        by_dest[NodeID::from(neighbor) as usize] = Some(viaref);
                        reachable += 1
                    }
                    if let Some(via) = &path.via
                        && !visited[NodeID::from(node) as usize].contains(&via.contact)
                    {
                        visited[NodeID::from(node) as usize].insert(via.contact);
                        all.push(path);
                        prioqueue.insert(((path, neighbor), ()), multigraph, bundle);
                    }
                }
            }
        }
        let mut elided = Vec::with_capacity(multigraph.get_rnode_count());
        let mut new_indexs = vec![None; all.capacity()];
        for (i, path_opt) in by_dest.into_iter().enumerate() {
            if let Some(idx) = path_opt {
                new_indexs[idx] = Some(i);
                let path = all[idx];
                elided.push(Some(path));
            } else {
                elided.push(None);
            }
        }
        for i in 0..multigraph.get_rnode_count() {
            if let Some(mut frag) = elided[i] {
                let mut index = i;
                loop {
                    if let Some(via) = frag.via.as_mut() {
                        if let Some(new_idx) = new_indexs[via.parent_frag] {
                            via.parent_frag = new_idx;
                            elided[index] = Some(frag);
                            break;
                        } else {
                            let old_idx = via.parent_frag;
                            let new_idx = elided.len();
                            elided.push(None);
                            via.parent_frag = new_idx;
                            frag = all[old_idx];
                            index = new_idx;
                        }
                    } else {
                        elided[index] = Some(frag);
                        break;
                    }
                }
            } else {
                continue;
            }
        }

        Ok(Some(PathFindingOutput {
            path_tree: Either::Right(elided),
        }))
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
