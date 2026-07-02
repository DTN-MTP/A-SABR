extern crate alloc;

use core::marker::PhantomData;

use crate::{
    bundle::Bundle, contact_manager::ContactManager, errors::ASABRError, node_manager::NodeManager,
    pathfinding::PathFindingOutput, types::Date,
};

use super::PathsStorage;

/// A cache for storing pathfinding output entries, enabling efficient retrieval and management.
///
/// The `Cache` struct provides a mechanism to store multiple `PathFindingOutput` instances
/// while enforcing limits on the number of entries based on size and priority checks.
#[derive(Debug)]
pub struct TreeCache<'id, NM: NodeManager, CM: ContactManager> {
    _phantom_nm: PhantomData<fn(&'id (), NM, CM)>,
}
impl<'id, NM: NodeManager, CM: ContactManager> TreeCache<'id, NM, CM> {
    pub fn new(_multigrap: &crate::multigraph::Multigraph<'id, NM, CM>) -> Self {
        Self {
            _phantom_nm: PhantomData,
        }
    }
}

impl<'id, NM: NodeManager, CM: ContactManager> PathsStorage<'id, NM, CM>
    for TreeCache<'id, NM, CM>
{
    fn select<'a>(
        &'a mut self,
        _bundle: &Bundle,
        _route_time: Date,
        _curr_time: Option<Date>,
        _multigraph: &crate::multigraph::Multigraph<'id, NM, CM>,
    ) -> Result<Option<PathFindingOutput<'id, 'a>>, ASABRError> {
        Ok(None)
    }

    fn store<'a>(
        &'a mut self,
        _bundle: &Bundle,
        tree: PathFindingOutput<'id, '_>,
    ) -> PathFindingOutput<'id, 'a> {
        tree.clone()
    }
    // /// Loads a pathfinding output from the cache that matches the provided bundle and excluded nodes.
    // ///
    // /// # Parameters
    // ///
    // /// * `bundle` - A reference to the `Bundle` containing routing information.
    // /// * `curr_time` - The current time.
    // /// * `excluded_nodes_sorted` - A sorted vector of `NodeID`s representing nodes to exclude from pathfinding.
    // ///
    // /// # Returns
    // ///
    // /// * `(Option<Rc<RefCell<PathFindingOutput<NM, CM>>>>, Option<Vec<NodeID>>)` - An optional reference-counted and mutable reference
    // ///   to the `PathFindingOutput` if a match is found; and the list of reached nodes if applicable (multicast).
    // fn select(
    //     &self,
    //     bundle: &Bundle,
    //     curr_time: Date,
    //     excluded_nodes_sorted: &[NodeID],
    // ) -> Result<
    //     (
    //         Option<Rc<RefCell<PathFindingOutput<NM, CM>>>>,
    //         Option<Vec<NodeID>>,
    //     ),
    //     ASABRError,
    // > {
    //     let multicast = bundle.destinations.len() > 1;
    //     for tree in &self.trees {
    //         if tree
    //             .borrow()
    //             .bundle
    //             .shadows(bundle, self.check_size, self.check_priority)
    //         {
    //             continue;
    //         }
    //         if tree.borrow().excluded_nodes_sorted != excluded_nodes_sorted {
    //             continue;
    //         }
    //         match multicast {
    //             false => {
    //                 if let Some(_res) =
    //                     dry_run_unicast_tree(bundle, curr_time, tree.clone(), false)?
    //                 {
    //                     return Ok((Some(tree.clone()), None));
    //                 }
    //             }
    //             true => {
    //                 let reachable_nodes = dry_run_multicast(bundle, curr_time, tree.clone())?;
    //                 return Ok((Some(tree.clone()), Some(reachable_nodes)));
    //             }
    //         }
    //     }
    //     Ok((None, None))
    // }

    // /// Stores a pathfinding output tree in the cache. Replaces a tree for a known exclusion list.
    // ///
    // /// If the cache exceeds its maximum entry limit, the oldest entry is removed.
    // ///
    // /// # Parameters
    // ///
    // /// * `new_tree` - A reference-counted mutable reference to the `PathfindingOutput` to store.
    // fn store(&mut self, _bundle: &Bundle, new_tree: Rc<RefCell<PathFindingOutput<NM, CM>>>) {
    //     let mut replace_index = None;
    //     for (i, tree) in self.trees.iter().enumerate() {
    //         if tree.borrow().excluded_nodes_sorted == new_tree.borrow().excluded_nodes_sorted {
    //             replace_index = Some(i);
    //             break;
    //         }
    //     }

    //     if let Some(i) = replace_index {
    //         self.trees[i] = new_tree;
    //     } else {
    //         self.trees.push_back(new_tree);
    //     }

    //     if self.trees.len() > self.max_entries {
    //         self.trees.pop_front();
    //     }
    // }
}
