use std::{cell::RefCell, collections::HashMap, rc::Rc};

pub mod cache;
pub mod table;

use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    distance::Distance,
    node::Node,
    node_manager::NodeManager,
    pathfinding::PathFindingOutput,
    route_stage::RouteStage,
    types::{Date, NodeID, Priority, Volume},
};

/// A trait for managing tree storage and retrieval.
///
/// This trait defines methods for loading and storing pathfinding output
/// related to routes in a routing system. Implementers of this trait must
/// provide their own logic for handling route data.
pub trait TreeStorage<NM: NodeManager, CM: ContactManager, D: Distance<CM>> {
    /// Loads the pathfinding output for a specific bundle, considering excluded nodes.
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
    /// * `Option<Rc<RefCell<PathfindingOutput<CM>>>>` - An optional reference-counted and mutable reference
    ///   to the `PathfindingOutput` if it exists; otherwise, returns `None`.
    fn select(
        &self,
        bundle: &Bundle,
        curr_time: Date,
        node_list: &Vec<Rc<RefCell<Node<NM>>>>,
        excluded_nodes_sorted: &Vec<NodeID>,
    ) -> (
        Option<Rc<RefCell<PathFindingOutput<CM, D>>>>,
        Option<Vec<NodeID>>,
    );

    /// Stores the pathfinding output tree for future use.
    ///
    /// # Parameters
    /// * `bundle` - A bundle copy for which the tree was created.
    /// * `tree` - A reference-counted mutable reference to the `PathfindingOutput` to store.
    fn store(&mut self, bundle: &Bundle, tree: Rc<RefCell<PathFindingOutput<CM, D>>>);
}

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Route<CM: ContactManager, D: Distance<CM>> {
    pub source_stage: Rc<RefCell<RouteStage<CM, D>>>,
    pub destination_stage: Rc<RefCell<RouteStage<CM, D>>>,
}

impl<CM: ContactManager, D: Distance<CM>> Route<CM, D> {
    pub fn from_tree(tree: Rc<RefCell<PathFindingOutput<CM, D>>>, dest: NodeID) -> Option<Self> {
        let tree_ref = tree.borrow();
        let source_stage = tree_ref.get_source_route();
        if tree_ref.by_destination[dest as usize] == None {
            return None;
        }
        if let Some(destination_stage) = tree_ref.by_destination[dest as usize].clone() {
            return Some(Route {
                source_stage,
                destination_stage,
            });
        }

        return None;
    }
}

impl<CM: ContactManager, D: Distance<CM>> Clone for Route<CM, D> {
    fn clone(&self) -> Self {
        Route {
            source_stage: Rc::clone(&self.source_stage),
            destination_stage: Rc::clone(&self.destination_stage),
        }
    }
}

/// A trait for managing route storage and retrieval.
///
/// This trait defines methods for loading and storing pathfinding output
/// related to routes in a routing system. Implementers of this trait must
/// provide their own logic for handling route data.
pub trait RouteStorage<NM: NodeManager, CM: ContactManager, D: Distance<CM>> {
    /// Loads the pathfinding output for a specific bundle, considering excluded nodes.
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
    /// * `Option<Route<CM, D>>` - An optional reference-counted and mutable reference
    ///   to the `Route` if it exists; otherwise, returns `None`.
    fn select(
        &mut self,
        bundle: &Bundle,
        curr_time: Date,
        node_list: &Vec<Rc<RefCell<Node<NM>>>>,
        excluded_nodes_sorted: &Vec<NodeID>,
    ) -> Option<Route<CM, D>>;

    fn store(&mut self, bundle: &Bundle, route: Route<CM, D>);
}

/// A struct that manages limits and conditions for scheduling based on bundle characteristics.
///
/// The `Guard` struct keeps track of known routing limits and determines if a scheduling
/// should be aborted based on its properties and the properties of the associated `Bundle`.
pub struct Guard {
    with_priorities: bool,
    known_limits: HashMap<(NodeID, Priority), Volume>,
}

impl Guard {
    /// Creates a new `Guard` instance with specified priority handling.
    ///
    /// # Parameters
    ///
    /// * `with_priorities` - A boolean indicating whether to consider priorities in the guard logic.
    ///
    /// # Returns
    ///
    /// * `Self` - A new instance of `Guard`.
    pub fn new(with_priorities: bool) -> Self {
        Self {
            with_priorities,
            known_limits: HashMap::new(),
        }
    }

    /// Determines whether the processing must be aborted based on the known limits and bundle.
    ///
    /// This method checks if the current `Bundle` cannot reach any destinations due to size limits.
    ///
    /// # Parameters
    ///
    /// * `bundle` - A reference to the `Bundle` being evaluated.
    ///
    /// # Returns
    ///
    /// * `bool` - Returns `true` if processing must be aborted; otherwise, returns `false`.
    pub fn must_abort(&self, bundle: &Bundle) -> bool {
        let priority = if self.with_priorities {
            bundle.priority
        } else {
            0
        };
        let mut unreachable_count: usize = 0;

        for dest in &bundle.destinations {
            if let Some(limit) = self.known_limits.get(&(*dest, priority)) {
                if bundle.size < *limit {
                    unreachable_count += 1;
                }
            }
        }
        unreachable_count == bundle.destinations.len()
    }

    /// Adds a new size limit for a specific destination based on the given bundle.
    ///
    /// If the new size limit is larger than the current limit for the destination and priority,
    /// it updates the known limits.
    ///
    /// # Parameters
    ///
    /// * `bundle` - A reference to the `Bundle` containing the size to be added.
    /// * `dest` - The destination `NodeID` for which the limit is being added.
    pub fn add_limit(&mut self, bundle: &Bundle, dest: NodeID) {
        let priority = if self.with_priorities {
            bundle.priority
        } else {
            0
        };
        if let Some(val) = self.known_limits.get(&(dest, priority)) {
            if val <= &bundle.size {
                return;
            }
        }
        self.known_limits.insert((dest, priority), bundle.size);
    }
}
