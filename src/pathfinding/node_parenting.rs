use std::{
    cell::RefCell, cmp::Ordering, cmp::Reverse, collections::BinaryHeap, marker::PhantomData,
    rc::Rc,
};

use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    distance::{Distance, DistanceWrapper},
    errors::ASABRError,
    multigraph::Multigraph,
    node_manager::NodeManager,
    route_stage::RouteStage,
    types::{Date, NodeID},
};

use super::{PathFindingOutput, Pathfinding, try_make_hop};

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
            /// Constructs a new `NodeParenting` instance with the provided nodes and contacts.
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
            /// * `excluded_nodes_sorted` - A sorted list of `NodeID`s to be excluded from the pathfinding.
            ///
            /// # Returns
            ///
            /// * `Result<PathFindingOutput<NM, CM>, ASABRError>` - The resulting pathfinding output, including the routes found,
            ///   or an error if the operation fails.
            fn get_next(
                &mut self,
                current_time: Date,
                source: NodeID,
                bundle: &Bundle,
                excluded_nodes_sorted: &[NodeID],
            ) -> Result<PathFindingOutput<NM, CM>, ASABRError> {
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
                            && let Some(route_proposition) = try_make_hop(
                                first_contact_index,
                                &from_route,
                                bundle,
                                &receiver.contacts_to_receiver,
                                &sender.node,
                                &receiver.node,
                            )
                        {
                            let idx = receiver.node.borrow().info.id as usize;
                            let push = match tree.by_destination[idx].as_ref() {
                                Some(known_route_ref) => {
                                    let mut known_route = known_route_ref.try_borrow_mut()?;
                                    if D::cmp(&route_proposition, &known_route) == Ordering::Less {
                                        known_route.is_disabled = true;
                                        true
                                    } else {
                                        false
                                    }
                                }
                                None => true,
                            };

                            if push {
                                let route_ref = Rc::new(RefCell::new(route_proposition));
                                tree.by_destination[idx] = Some(route_ref.clone());
                                priority_queue.push(Reverse(DistanceWrapper::new(route_ref)));
                            }
                        }
                    }
                }

                Ok(tree)
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

define_node_graph!(NodeParentingTreeExcl, true, true);
define_node_graph!(NodeParentingPath, false, false);
define_node_graph!(NodeParentingPathExcl, false, true);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::Bundle;
    use crate::contact::Contact;
    use crate::contact::ContactInfo;
    use crate::contact_manager::legacy::evl::EVLManager;
    use crate::distance::hop::Hop;
    use crate::distance::sabr::SABR;
    use crate::multigraph::Multigraph;
    use crate::node::Node;
    use crate::node::NodeInfo;
    use crate::node_manager::none::NoManagement;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn make_node(id: u16, name: &str) -> Node<NoManagement> {
        Node::try_new(
            NodeInfo {
                id,
                name: name.into(),
                excluded: false,
            },
            NoManagement {},
        )
        .unwrap()
    }

    fn make_contact(
        tx: u16,
        rx: u16,
        start: f64,
        end: f64,
        rate: f64,
        delay: f64,
    ) -> Contact<NoManagement, EVLManager> {
        Contact::try_new(
            ContactInfo::new(tx, rx, start, end),
            EVLManager::new(rate, delay),
        )
        .unwrap_or_else(|| panic!("Contact failed"))
    }

    fn make_bundle(dest: NodeID, priority: i8, size: f64, expiration: f64) -> Bundle {
        Bundle {
            source: 0,
            destinations: vec![dest],
            priority,
            size,
            expiration,
        }
    }

    fn assert_time_hop(
        res: &PathFindingOutput<NoManagement, EVLManager>,
        dest: usize,
        expected_time: f64,
        expected_hop: u16,
        distance: &str,
    ) {
        let r = res.by_destination[dest]
            .as_ref()
            .unwrap_or_else(|| panic!("{distance} : No route found to node {dest}"))
            .borrow();
        assert_eq!(
            r.at_time, expected_time,
            "{distance} : Arrival time should be {expected_time}"
        );
        assert_eq!(
            r.hop_count, expected_hop,
            "{distance} : Should be {expected_hop} hops"
        );
    }

    fn unit_graph_test() -> Rc<RefCell<Multigraph<NoManagement, EVLManager>>> {
        Rc::new(RefCell::new(Multigraph::new(
            vec![make_node(0, "A"), make_node(1, "B"), make_node(2, "C")],
            vec![
                make_contact(0, 1, 0.0, 2000.0, 100.0, 1.0),
                make_contact(1, 2, 0.0, 2000.0, 100.0, 1.0),
            ],
        )))
    }

    fn five_contact_graph_test() -> Rc<RefCell<Multigraph<NoManagement, EVLManager>>> {
        Rc::new(RefCell::new(Multigraph::new(
            vec![
                make_node(0, "A"),
                make_node(1, "B"),
                make_node(2, "C"),
                make_node(3, "D"),
            ],
            vec![
                make_contact(0, 1, 0.0, 2000.0, 100.0, 0.01),
                make_contact(1, 2, 0.0, 2000.0, 100.0, 1.0),
                make_contact(0, 3, 0.0, 2000.0, 100.0, 0.1),
                make_contact(3, 2, 0.0, 2000.0, 100.0, 0.01),
                make_contact(0, 2, 0.0, 2000.0, 100.0, 10.0),
            ],
        )))
    }

    fn exemple_1_graph() -> Rc<RefCell<Multigraph<NoManagement, EVLManager>>> {
        Rc::new(RefCell::new(Multigraph::new(
            vec![
                make_node(0, "source"),
                make_node(1, "from_C0"),
                make_node(2, "from_C2_C1"),
                make_node(3, "from_C3"),
            ],
            vec![
                make_contact(0, 1, 0.0, 10.0, 1.0, 0.0),
                make_contact(0, 2, 25.0, 35.0, 1.0, 0.0),
                make_contact(1, 2, 10.0, 20.0, 1.0, 0.0),
                make_contact(2, 3, 30.0, 40.0, 1.0, 0.0),
            ],
        )))
    }

    fn exemple_2_graph() -> Rc<RefCell<Multigraph<NoManagement, EVLManager>>> {
        Rc::new(RefCell::new(Multigraph::new(
            vec![
                make_node(0, "source"),
                make_node(1, "from_C0"),
                make_node(2, "from_C2_C1"),
                make_node(3, "from_C3"),
                make_node(4, "from_C4"),
            ],
            vec![
                make_contact(0, 1, 0.0, 10.0, 1.0, 0.0),
                make_contact(0, 2, 25.0, 35.0, 1.0, 0.0),
                make_contact(1, 2, 10.0, 20.0, 1.0, 0.0),
                make_contact(2, 3, 20.0, 40.0, 1.0, 0.0),
                make_contact(3, 4, 50.0, 60.0, 1.0, 0.0),
            ],
        )))
    }

    #[test]
    fn test_a_to_c_tree() {
        let mg = unit_graph_test();

        let mut algo_hop = NodeParentingTreeExcl::<NoManagement, EVLManager, Hop>::new(mg.clone());
        let mut algo_sabr =
            NodeParentingTreeExcl::<NoManagement, EVLManager, SABR>::new(mg.clone());

        let bundle = make_bundle(2, 1, 1.0, 2000.0);

        let res_hop = algo_hop
            .get_next(0.0, 0, &bundle, &[][..])
            .expect("Hop : Routing Failed !");

        assert_time_hop(&res_hop, 2, 2.02, 2, "Hop");

        let res_sabr = algo_sabr
            .get_next(0.0, 0, &bundle, &[][..])
            .expect("SABR : Routing Failed !");

        assert_time_hop(&res_sabr, 2, 2.02, 2, "SABR");
    }

    #[test]
    fn test_a_to_c_tree_excluded() {
        let mg = unit_graph_test();

        let mut algo_hop = NodeParentingTreeExcl::<NoManagement, EVLManager, Hop>::new(mg.clone());
        let mut algo_sabr =
            NodeParentingTreeExcl::<NoManagement, EVLManager, SABR>::new(mg.clone());

        let bundle = make_bundle(2, 1, 1.0, 2000.0);
        let excluded = [1];

        let res_hop = algo_hop
            .get_next(0.0, 0, &bundle, &excluded[..])
            .expect("Hop : Routing Failed !");
        assert!(
            res_hop.by_destination[1].is_none(),
            "Hop : B should be excluded"
        );
        assert!(
            res_hop.by_destination[2].is_none(),
            "Hop : C should not be accessible without B"
        );

        let res_sabr = algo_sabr
            .get_next(0.0, 0, &bundle, &excluded[..])
            .expect("SABR : Routing Failed !");
        assert!(
            res_sabr.by_destination[1].is_none(),
            "SABR : B should be excluded"
        );
        assert!(
            res_sabr.by_destination[2].is_none(),
            "SABR : C should not be accessible without B"
        );
    }

    #[test]
    fn test_a_to_c_path_excl() {
        let mg = unit_graph_test();

        let mut algo_hop = NodeParentingPathExcl::<NoManagement, EVLManager, Hop>::new(mg.clone());
        let mut algo_sabr =
            NodeParentingPathExcl::<NoManagement, EVLManager, SABR>::new(mg.clone());

        let bundle = make_bundle(2, 1, 1.0, 2000.0);
        let excluded = [1];

        let res_hop = algo_hop
            .get_next(0.0, 0, &bundle, &excluded[..])
            .expect("Hop : Routing Failed !");
        assert!(
            res_hop.by_destination[2].is_none(),
            "Hop : C should not be accessible without B"
        );

        let res_sabr = algo_sabr
            .get_next(0.0, 0, &bundle, &excluded[..])
            .expect("SABR : Routing Failed !");
        assert!(
            res_sabr.by_destination[2].is_none(),
            "SABR : C should not be accessible without B"
        );
    }

    #[test]
    fn test_two_paths_to_c() {
        let mg = five_contact_graph_test();

        let mut algo_hop = NodeParentingTreeExcl::<NoManagement, EVLManager, Hop>::new(mg.clone());
        let mut algo_sabr =
            NodeParentingTreeExcl::<NoManagement, EVLManager, SABR>::new(mg.clone());

        let bundle = make_bundle(2, 1, 1.0, 2000.0);

        let res_hop = algo_hop
            .get_next(0.0, 0, &bundle, &[][..])
            .expect("Hop : Routing Failed !");

        assert_time_hop(&res_hop, 2, 10.01, 1, "Hop");

        let res_sabr = algo_sabr
            .get_next(0.0, 0, &bundle, &[][..])
            .expect("SABR : Routing Failed !");

        assert_time_hop(&res_sabr, 2, 0.13, 2, "SABR");
    }

    #[test]
    fn test_exemple_1() {
        let mg = exemple_1_graph();

        let mut algo_hop = NodeParentingPath::<NoManagement, EVLManager, Hop>::new(mg.clone());
        let mut algo_sabr = NodeParentingPath::<NoManagement, EVLManager, SABR>::new(mg.clone());

        let bundle = make_bundle(3, 0, 0.0, 1000.0);

        let res_hop = algo_hop
            .get_next(0.0, 0, &bundle, &[][..])
            .expect("Routing Failed !");

        assert_time_hop(&res_hop, 3, 30.0, 2, "Hop");

        let res_sabr = algo_sabr
            .get_next(0.0, 0, &bundle, &[][..])
            .expect("Routing Failed !");

        assert_time_hop(&res_sabr, 3, 30.0, 3, "SABR");
    }

    #[test]
    fn test_exemple_2() {
        let mg = exemple_2_graph();

        let mut algo_hop = NodeParentingPath::<NoManagement, EVLManager, Hop>::new(mg.clone());
        let mut algo_sabr = NodeParentingPath::<NoManagement, EVLManager, SABR>::new(mg.clone());

        let bundle = make_bundle(4, 0, 0.0, 1000.0);

        let res_hop = algo_hop
            .get_next(0.0, 0, &bundle, &[][..])
            .expect("Routing Failed !");

        assert_time_hop(&res_hop, 4, 50.0, 3, "Hop");

        let res_sabr = algo_sabr
            .get_next(0.0, 0, &bundle, &[][..])
            .expect("Routing Failed !");

        assert_time_hop(&res_sabr, 4, 50.0, 4, "SABR");
    }
}
