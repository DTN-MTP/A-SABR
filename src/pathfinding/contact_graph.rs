use std::{
    cell::RefCell,
    cmp::{Ordering, Reverse},
    collections::BinaryHeap,
    marker::PhantomData,
    rc::Rc,
};

use crate::{
    bundle::Bundle,
    contact::Contact,
    contact_manager::ContactManager,
    distance::Distance,
    multigraph::Multigraph,
    node_manager::NodeManager,
    route_stage::RouteStage,
    types::{Date, NodeID},
};

use super::{route_stage_distance::DistanceWrapper, try_make_hop, PathFindingOutput, Pathfinding};

/// Attempts to update the current work area of a contact if the new route proposition is "closer".
///
/// This function checks if the new route proposition is an improvement over the current work area
/// for a given contact. If it is, the work area is updated and a reference to it is returned.
///
/// # Type Parameters
/// * `CM`: A type implementing the `ContactManager` trait, representing the contact management
///         system for the route.
/// * `D`: A type implementing the `Distance` trait, providing the distance metric used to
///        compare routes.
///
/// # Parameters
///
/// * `route_proposition` - A `RouteStage` that proposes a new route for a contact.
///
/// # Returns
///
/// * `Option<Rc<RefCell<RouteStage<CM>>>>` - An optional reference to the updated work area if the proposition is "closer".
fn update_if_closer<CM: ContactManager, D: Distance<CM>>(
    route_proposition: RouteStage<CM>,
) -> Option<Rc<RefCell<RouteStage<CM>>>> {
    if let Some(via) = route_proposition.via.as_ref() {
        let contact_ref = via.contact.borrow_mut();
        let mut current_work_area = contact_ref.work_area.borrow_mut();
        {
            if !(D::cmp(&route_proposition, &current_work_area) == Ordering::Less) {
                return None;
            }
        }
        current_work_area.update_with(&route_proposition);
        return Some(contact_ref.work_area.clone());
    }
    // This will never happen (the proposition shall always have a via_contact)
    return None;
}

macro_rules! define_contact_graph {
    ($name:ident, $is_tree_output:tt, $with_exclusions:tt) => {
        /// A contact parenting (contact graph) implementation of Dijkstra algorithm.
        ///
        /// This implementation includes shortest-path tree construction.
        ///
        /// # Type Parameters
        ///
        /// * `NM` - A type that implements the `NodeManager` trait.
        /// * `CM` - A type that implements the `ContactManager` trait.
        pub struct $name<NM: NodeManager, CM: ContactManager, D: Distance<CM>> {
            /// The node multigraph for contact access.
            graph: Rc<RefCell<Multigraph<NM, CM>>>,
            /// For tree construction, tracks the nodes visited as transmitters.
            visited_as_tx_ids: Vec<bool>,
            /// For tree construction, tracks the nodes visited as receivers.
            visited_as_rx_ids: Vec<bool>,
            /// For tree construction, tracks the count of nodes visited as transmitters.
            visited_as_tx_count: usize,
            /// For tree construction, tracks the count of nodes visited as receivers.
            visited_as_rx_count: usize,

            #[doc(hidden)]
            _phantom_distance: PhantomData<D>,
        }

        impl<NM: NodeManager, CM: ContactManager, D: Distance<CM>> Pathfinding<NM, CM>
            for $name<NM, CM, D>
        {
            /// Constructs a new `ContactGraph` instance with the provided nodes and contacts.
            ///
            /// # Parameters
            ///
            /// * `multigraph` - A shared pointer to a multigraph.
            ///
            /// # Returns
            ///
            #[doc = concat!( " * `Self` - A new instance of `",stringify!($name),"`.")]
            fn new(multigraph: Rc<RefCell<Multigraph<NM, CM>>>) -> Self {
                let mut node_count: usize = 0;
                if $is_tree_output {
                    node_count = multigraph.borrow().get_node_count();
                }

                Self {
                    graph: multigraph,
                    visited_as_tx_ids: vec![false; node_count],
                    visited_as_rx_ids: vec![false; node_count],
                    visited_as_tx_count: 1,
                    visited_as_rx_count: 1,
                    _phantom_distance: PhantomData,
                }
            }

            /// Finds the next route based on the current state and available contacts.
            ///
            /// This method uses a priority queue to explore potential routes from a source node,
            /// considering the current time, bundle, and nodes to exclude from the pathfinding.
            ///
            /// # Parameters
            ///
            /// * `current_time` - The current time used for evaluating routes.
            /// * `source` - The `NodeID` of the source node from which to begin pathfinding.
            /// * `bundle` - The `Bundle` associated with the pathfinding operation.
            /// * `excluded_nodes_sorted` - A sorted list of `NodeID`s to be excluded from the pathfinding.
            ///
            /// # Returns
            ///
            /// * `PathfindingOutput<CM>` - The resulting pathfinding output, including the routes found.
            fn get_next(
                &mut self,
                current_time: Date,
                source: NodeID,
                bundle: &Bundle,
                excluded_nodes_sorted: &Vec<NodeID>,
            ) -> PathFindingOutput<CM> {
                let mut graph = self.graph.borrow_mut();
                if $with_exclusions {
                    graph.apply_exclusions_sorted(excluded_nodes_sorted);
                }
                let source_route: Rc<RefCell<RouteStage<CM>>> =
                    Rc::new(RefCell::new(RouteStage::new(current_time, source, None)));
                let mut tree: PathFindingOutput<CM> = PathFindingOutput::new(
                    &bundle,
                    source_route.clone(),
                    &excluded_nodes_sorted,
                    graph.senders.len(),
                );
                let mut priority_queue: BinaryHeap<Reverse<DistanceWrapper<CM, D>>> =
                    BinaryHeap::new();
                let mut altered_contacts: Vec<Rc<RefCell<Contact<CM>>>> = Vec::new();

                if $is_tree_output {
                    self.visited_as_tx_ids.fill(false);
                    self.visited_as_rx_ids.fill(false);
                    self.visited_as_tx_count = 1;
                    self.visited_as_rx_count = 1;
                    self.visited_as_tx_ids[source as usize] = true;
                    self.visited_as_rx_ids[source as usize] = true;
                }

                tree.by_destination[source as usize] = Some(source_route.clone());
                priority_queue.push(Reverse(DistanceWrapper::new(Rc::clone(&source_route))));

                while let Some(Reverse(DistanceWrapper(from_route, _))) = priority_queue.pop() {
                    let tx_node_id = from_route.borrow().to_node;

                    if !$is_tree_output {
                        if bundle.destinations[0] == tx_node_id {
                            break;
                        }
                    }

                    if $is_tree_output {
                        if !(self.visited_as_tx_ids[tx_node_id as usize]) {
                            self.visited_as_tx_ids[tx_node_id as usize] = true;
                            self.visited_as_tx_count += 1;
                        }
                    }

                    let sender = &mut graph.senders[tx_node_id as usize];

                    for receiver in &mut sender.receivers {
                        if $with_exclusions {
                            if receiver.is_excluded() {
                                continue;
                            }
                        }

                        if let Some(first_contact_index) =
                            receiver.lazy_prune_and_get_first_idx(current_time)
                        {
                            if let Some(route_proposition) = try_make_hop(
                                first_contact_index,
                                &from_route,
                                &bundle,
                                &receiver.contacts_to_receiver,
                                &sender.node,
                                &receiver.node,
                            ) {
                                if let Some(updated_route) =
                                    update_if_closer::<CM, D>(route_proposition)
                                {
                                    if let Some(via) = &updated_route.borrow().via {
                                        altered_contacts.push(via.contact.clone());
                                    }
                                    let rx_node_id = receiver.node.borrow().info.id;
                                    priority_queue
                                        .push(Reverse(DistanceWrapper::new(updated_route.clone())));
                                    tree.by_destination[rx_node_id as usize] = Some(updated_route);

                                    if $is_tree_output {
                                        if !(self.visited_as_rx_ids[rx_node_id as usize]) {
                                            self.visited_as_rx_ids[rx_node_id as usize] = true;
                                            self.visited_as_rx_count += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if $is_tree_output {
                        if self.visited_as_tx_count == self.visited_as_rx_count {
                            break;
                        }
                    }
                }

                // We replace rather than clear because some work areas became part of the output.
                for contact in altered_contacts {
                    let to_node = contact.borrow().get_rx_node();
                    let new_work_area = Rc::new(RefCell::new(RouteStage::new_work_area(to_node)));
                    RefCell::borrow_mut(&contact).work_area = new_work_area;
                }

                return tree;
            }

            /// Get a shared pointer to the multigraph.
            ///
            /// # Returns
            ///
            /// * A shared pointer to the multigraph.
            fn get_multigraph(&self) -> Rc<RefCell<Multigraph<NM, CM>>> {
                return self.graph.clone();
            }
        }
    };
}

define_contact_graph!(ContactGraphTree, true, true);
define_contact_graph!(ContactGraphPath, false, false);
