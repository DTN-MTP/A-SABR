use std::{cell::RefCell, collections::VecDeque, marker::PhantomData, rc::Rc};

use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    distance::Distance,
    node::Node,
    node_manager::NodeManager,
    pathfinding::PathFindingOutput,
    routing::{dry_run_multicast, dry_run_unicast_tree},
    types::{Date, NodeID},
};

use super::TreeStorage;

/// A cache for storing pathfinding output entries, enabling efficient retrieval and management.
///
/// The `Cache` struct provides a mechanism to store multiple `PathfindingOutput` instances
/// while enforcing limits on the number of entries based on size and priority checks.
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct TreeCache<NM: NodeManager, CM: ContactManager, D: Distance<CM>> {
    /// A boolean indicating whether to check the size of bundles in the cache.
    check_size: bool,
    /// A boolean indicating whether to check the priority of bundles in the cache.
    check_priority: bool,
    /// The maximum number of entries allowed in the cache.
    max_entries: usize,
    /// A deque of reference-counted mutable references to `PathfindingOutput` instances stored in the cache.
    trees: VecDeque<Rc<RefCell<PathFindingOutput<CM, D>>>>,

    // for compilation
    #[doc(hidden)]
    _phantom_nm: PhantomData<NM>,
}

impl<NM: NodeManager, CM: ContactManager, D: Distance<CM>> TreeCache<NM, CM, D> {
    /// Creates a new `Cache` instance with specified entry management settings.
    ///
    /// # Parameters
    ///
    /// * `check_size` - A boolean indicating whether to check the size of bundles in the cache.
    /// * `check_priority` - A boolean indicating whether to check the priority of bundles in the cache.
    /// * `max_entries` - The maximum number of entries allowed in the cache.
    ///
    /// # Returns
    ///
    /// * `Self` - A new instance of `Cache`.
    pub fn new(check_size: bool, check_priority: bool, max_entries: usize) -> Self {
        Self {
            check_size,
            check_priority,
            max_entries,
            trees: VecDeque::new(),
            // for compilation
            _phantom_nm: PhantomData,
        }
    }
}

impl<NM: NodeManager, CM: ContactManager, D: Distance<CM>> TreeStorage<NM, CM, D>
    for TreeCache<NM, CM, D>
{
    /// Loads a pathfinding output from the cache that matches the provided bundle and excluded nodes.
    ///
    /// # Parameters
    ///
    /// * `bundle` - A reference to the `Bundle` containing routing information.
    /// * `curr_time` - The current time.
    /// * `node_list` - The list of node objects.
    /// * `excluded_nodes_sorted` - A sorted vector of `NodeID`s representing nodes to exclude from pathfinding.
    ///
    /// # Returns
    ///
    /// * `(Option<Rc<RefCell<PathFindingOutput<CM, D>>>>,Option<Vec<NodeID>>,)` - An optional reference-counted and mutable reference
    ///   to the `PathfindingOutput` if a match is found; and the list of reached nodes if applicable (multicast).
    fn select(
        &self,
        bundle: &Bundle,
        curr_time: Date,
        node_list: &Vec<Rc<RefCell<Node<NM>>>>,
        excluded_nodes_sorted: &Vec<NodeID>,
    ) -> (
        Option<Rc<RefCell<PathFindingOutput<CM, D>>>>,
        Option<Vec<NodeID>>,
    ) {
        let multicast = bundle.destinations.len() > 1;
        for tree in &self.trees {
            if tree
                .borrow()
                .bundle
                .shadows(bundle, self.check_size, self.check_priority)
            {
                continue;
            }
            if &tree.borrow().excluded_nodes_sorted != excluded_nodes_sorted {
                continue;
            }
            match multicast {
                false => {
                    if let Some(_res) =
                        dry_run_unicast_tree(bundle, curr_time, tree.clone(), node_list)
                    {
                        return (Some(tree.clone()), None);
                    }
                }
                true => {
                    let mut reachable_nodes = Vec::new();
                    dry_run_multicast(
                        bundle,
                        curr_time,
                        tree.clone(),
                        &mut reachable_nodes,
                        node_list,
                    );
                    return (Some(tree.clone()), Some(reachable_nodes));
                }
            }
        }
        (None, None)
    }

    /// Stores a pathfinding output tree in the cache. Replaces a tree for a known exclusion list.
    ///
    /// If the cache exceeds its maximum entry limit, the oldest entry is removed.
    ///
    /// # Parameters
    ///
    /// * `new_tree` - A reference-counted mutable reference to the `PathfindingOutput` to store.
    fn store(&mut self, _bundle: &Bundle, new_tree: Rc<RefCell<PathFindingOutput<CM, D>>>) {
        let mut replace_index = None;
        for (i, tree) in self.trees.iter().enumerate() {
            if tree.borrow().excluded_nodes_sorted == new_tree.borrow().excluded_nodes_sorted {
                replace_index = Some(i);
                break;
            }
        }

        if let Some(i) = replace_index {
            self.trees[i] = new_tree;
        } else {
            self.trees.push_back(new_tree);
        }

        if self.trees.len() > self.max_entries {
            self.trees.pop_front();
        }
    }
}
