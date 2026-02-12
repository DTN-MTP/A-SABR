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
    distance::{Distance, DistanceWrapper},
    multigraph::Multigraph,
    node_manager::NodeManager,
    route_stage::RouteStage,
    types::{Date, NodeID},
};

use super::{try_make_hop, PathFindingOutput, Pathfinding};

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
        pub struct $name<NM: NodeManager, CM: ContactManager, D: Distance<NM, CM>> {
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

        impl<NM: NodeManager, CM: ContactManager, D: Distance<NM, CM>> Pathfinding<NM, CM>
            for $name<NM, CM, D>
        {
            /// Constructs a new `ContactParenting` instance with the provided nodes and contacts.
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
                excluded_nodes_sorted: &[NodeID],
            ) -> Result<PathFindingOutput<NM, CM>, crate::errors::ASABRError> {
                let mut graph = self.graph.try_borrow_mut()?;
                if $with_exclusions {
                    graph.prepare_for_exclusions_sorted(excluded_nodes_sorted)?;
                }
                let source_route: Rc<RefCell<RouteStage<NM, CM>>> =
                    Rc::new(RefCell::new(RouteStage::new(
                        current_time,
                        source,
                        None,
                        #[cfg(feature = "node_proc")]
                        bundle.clone(),
                    )));

                let mut tree: PathFindingOutput<NM, CM> = PathFindingOutput::new(
                    &bundle,
                    source_route.clone(),
                    &excluded_nodes_sorted,
                    graph.senders.len(),
                );
                let mut priority_queue: BinaryHeap<Reverse<DistanceWrapper<NM, CM, D>>> =
                    BinaryHeap::new();
                let mut altered_contacts: Vec<Rc<RefCell<Contact<NM, CM>>>> = Vec::new();

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
                    if from_route.borrow().is_disabled {
                        continue;
                    }
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
                                let mut push = false;
                                if let Some(hop) = &route_proposition.via {
                                    // todo : improve CF..
                                    if let Some(know_route_ref) = &hop.contact.borrow().work_area {
                                        let mut know_route = know_route_ref.try_borrow_mut()?;
                                        if D::cmp(&route_proposition, &know_route) == Ordering::Less
                                        {
                                            // if "Test"
                                            know_route.is_disabled = true;
                                            push = true;
                                        }
                                    } else {
                                        // if "None"
                                        altered_contacts.push(hop.contact.clone());
                                        push = true;
                                    }
                                }
                                if push {
                                    let rx_node_id = receiver.node.borrow().info.id;

                                    if let Some(hop) = &route_proposition.via {
                                        let route_proposition_ref = Rc::new(RefCell::new(
                                            route_proposition.clone_work_area(),
                                        ));
                                        priority_queue.push(Reverse(DistanceWrapper::new(
                                            route_proposition_ref.clone(),
                                        )));
                                        let contact = &hop.contact;
                                        contact.try_borrow_mut()?.work_area =
                                            Some(route_proposition_ref.clone());

                                        // We can do this directly only in the if "Test" without the else
                                        if let Some(know_route_ref) =
                                            tree.by_destination[rx_node_id as usize].clone()
                                        {
                                            let known_best_route =
                                                know_route_ref.try_borrow_mut()?;
                                            if D::cmp(&route_proposition, &known_best_route)
                                                == Ordering::Less
                                            {
                                                tree.by_destination[rx_node_id as usize] =
                                                    Some(route_proposition_ref);
                                            }
                                        } else {
                                            // We can do this directly in the if "None"
                                            tree.by_destination[rx_node_id as usize] =
                                                Some(route_proposition_ref);
                                        }

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
                    }
                    if $is_tree_output {
                        if self.visited_as_tx_count == self.visited_as_rx_count {
                            break;
                        }
                    }
                }

                // We replace rather than clear because some work areas became part of the output.
                for contact in altered_contacts {
                    contact.try_borrow_mut()?.work_area = None;
                }

                return Ok(tree);
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

define_contact_graph!(ContactParentingTreeExcl, true, true);
define_contact_graph!(ContactParentingPath, false, false);
define_contact_graph!(ContactParentingPathExcl, false, true);
