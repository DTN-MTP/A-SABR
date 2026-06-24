extern crate alloc;
use alloc::{collections::BTreeSet, vec, vec::Vec};
use core::marker::PhantomData;

use crate::{
    bundle::Bundle, contact_manager::ContactManager, distance::Distance, multigraph::{ContactRef, Multigraph}, node_manager::NodeManager, pathfinding::{disktra::{Disktra, DisktraWorkspace}, flatten}, paths::{PathFragment, ViaHop}, types::NodeID
};

use super::super::PathFindingOutput ;

/// A contact parenting (contact graph) implementation of Dijkstra algorithm.
///
/// This implementation includes shortest-path tree construction.
///
/// # Type Parameters
///
/// * `NM` - A type that implements the `NodeManager` trait.
/// * `CM` - A type that implements the `ContactManager` trait.
pub type ContactParenting<
    'id,
    const tree_output: bool,
    NM: NodeManager,
    CM: ContactManager,
    D: Distance<NM, CM>,
> = Disktra<ContactParentingWorkArea<'id,NM,CM,D>,D>;

struct ContactParentingWorkArea<
    'id,
    NM: NodeManager,
    CM: ContactManager,
    D: Distance<NM,CM>
> {
    /// A vector storing all keeped path to a node without sorting for easy reference.
    possible_paths: Vec<PathFragment<'id>>,
    /// A vector containing (option of index of) pathfragment, to reach a given destination.
    by_destination: Vec<Option<usize>>,
    /// Visited contacts, grouped by node.
    visited: Vec<BTreeSet<ContactRef<'id>>>,
    _phantom: PhantomData<fn(NM, CM, D)>,
}

impl<'id, NM: NodeManager, CM: ContactManager, D: Distance<NM, CM>>
    DisktraWorkspace<'id,NM,CM> for ContactParentingWorkArea<'id,NM,CM,D>
{
    /// Constructs a new `ContactParenting` instance with the provided nodes and contacts.
    #[inline(always)]
    fn new(graph: &Multigraph<'id, NM, CM>) -> Self {
        Self {
            possible_paths: Vec::new(),
            by_destination: vec![None; graph.get_rnode_count()],
            visited : vec![BTreeSet::new(); graph.get_rnode_count()],
                    _phantom: PhantomData,
        }
    }
    fn into_pathfinding_output(self) -> PathFindingOutput<'id, 'static> {
        flatten(&self.possible_paths, self.by_destination.into_iter())
    }

    fn try_insert(
        &mut self,
        proposition: PathFragment<'id>,
        actual_node: crate::multigraph::RNodeRef<'id>,
        graph: &Multigraph<'id, NM, CM>,
        bundle: &Bundle,
    ) -> (Option<usize>, bool)
    {
        let new_idx = self.possible_paths.len();
        let route_for_node = &mut self.by_destination[NodeID::from(actual_node) as usize];
        let Some(ViaHop{contact,..}) = proposition.via else {
            if route_for_node.is_some(){
                return (None,false);
            }
            self.possible_paths.push(proposition);
            *route_for_node = Some(new_idx);
            return (Some(new_idx),true); 
        };
        let new = route_for_node.is_none();
        if new {
            *route_for_node = Some(new_idx);
        }
        if !self.visited[NodeID::from(actual_node) as usize].insert(contact)  {
            self.possible_paths.push(proposition);
            (Some(new_idx),new)
        } else {
            (None,false)
        }
    }

    fn node_check(&mut self, node: crate::multigraph::RNodeRef<'id>) -> bool {
        true
    }

    fn fragment_check(
        &mut self,
        proposition: PathFragment<'id>,
        dest_node: crate::multigraph::RNodeRef<'id>,
        graph: &Multigraph<'id, NM, CM>,
        bundle: &Bundle,
    ) -> bool
    {
        let Some(ViaHop { contact, ..}) = proposition.via else {
            return true;
        };
        !self.visited[NodeID::from(dest_node) as usize].contains(&contact)
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
