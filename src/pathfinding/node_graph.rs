use std::{
    cell::RefCell, cmp::Ordering, cmp::Reverse, collections::BinaryHeap, marker::PhantomData,
    rc::Rc,
};

use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    distance::{Distance, DistanceWrapper},
    multigraph::Multigraph,
    node_manager::NodeManager,
    route_stage::RouteStage,
    types::{Date, NodeID},
};

use super::{try_make_hop, PathFindingOutput, Pathfinding};

macro_rules! define_node_graph {
    ($name:ident, $is_tree_output:tt, $with_exclusions:tt) => {
        /// A node parenting (node graph, SPSN v1) implementation of Dijkstra algorithm.
        ///
        /// Use this implementation for optimized resource utilization.
        ///
        /// # Type Parameters
        ///
        /// * `NM` - A type that implements the `NodeManager` trait.
        /// * `CM` - A type that implements the `ContactManager` trait.
        /// * `D` - A type that implements the `Distance<NM, CM>` trait.
        pub struct $name<NM: NodeManager, CM: ContactManager, D: Distance<NM, CM>> {
            /// The node multigraph for contact access.
            graph: Rc<RefCell<Multigraph<NM, CM>>>,
            #[doc(hidden)]
            _phantom_distance: PhantomData<D>,
        }

        impl<NM: NodeManager, CM: ContactManager, D: Distance<NM, CM>> Pathfinding<NM, CM>
            for $name<NM, CM, D>
        {
            /// Constructs a new `NodeGraph` instance with the provided nodes and contacts.
            ///
            /// # Parameters
            ///
            /// * `multigraph` - A shared pointer to a multigraph.
            ///
            /// # Returns
            ///
            #[doc = concat!( " * `Self` - A new instance of `",stringify!($name),"`.")]
            fn new(multigraph: Rc<RefCell<Multigraph<NM, CM>>>) -> Self {
                Self {
                    graph: multigraph,
                    _phantom_distance: PhantomData,
                }
            }

            /// Finds the next route based on the current state and available contacts.
            ///
            /// This method uses a priority queue to explore potential routes from a source node,
            /// considering the current time, bundle, and excluded nodes.
            ///
            /// # Parameters
            ///
            /// * `current_time` - The current time used for evaluating routes.
            /// * `source` - The `NodeID` of the source node from which to begin pathfinding.
            /// * `bundle` - The `Bundle` associated with the pathfinding operation.
            /// * `excluded_nodes` - A list of `NodeID`s to be excluded from the pathfinding.
            ///
            /// # Returns
            ///
            /// * `PathfindingOutput<CM, D>` - The resulting pathfinding output, including the routes found.
            fn get_next(
                &mut self,
                current_time: Date,
                source: NodeID,
                bundle: &Bundle,
                excluded_nodes_sorted: &Vec<NodeID>,
            ) -> PathFindingOutput<NM, CM> {
                let mut graph = self.graph.borrow_mut();

                if $with_exclusions {
                    graph.prepare_for_exclusions_sorted(excluded_nodes_sorted);
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
                    bundle,
                    source_route.clone(),
                    excluded_nodes_sorted,
                    graph.senders.len(),
                );

                let mut priority_queue: BinaryHeap<Reverse<DistanceWrapper<NM, CM, D>>> =
                    BinaryHeap::new();

                for node_id in 0..graph.get_node_count() {
                    if node_id == source as usize {
                        tree.by_destination[node_id as usize] = Some(source_route.clone());
                    } else {
                        tree.by_destination[node_id as usize] = None;
                    }
                }

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
                                bundle,
                                &receiver.contacts_to_receiver,
                                &sender.node,
                                &receiver.node,
                            ) {
                                let mut push = false;
                                if let Some(know_route_ref) = tree.by_destination
                                    [receiver.node.borrow().info.id as usize]
                                    .clone()
                                {
                                    let mut known_route = know_route_ref.borrow_mut();
                                    if D::cmp(&route_proposition, &known_route) == Ordering::Less {
                                        known_route.is_disabled = true;
                                        push = true;
                                    }
                                } else {
                                    push = true;
                                }
                                if push {
                                    let route_ref = Rc::new(RefCell::new(route_proposition));
                                    tree.by_destination[receiver.node.borrow().info.id as usize] =
                                        Some(route_ref.clone());
                                    priority_queue.push(Reverse(DistanceWrapper::new(route_ref)));
                                }
                            }
                        }
                    }
                }

                tree
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

define_node_graph!(NodeGraphTreeExcl, true, true);
define_node_graph!(NodeGraphPath, false, false);
