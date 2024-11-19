use crate::{
    bundle::Bundle,
    contact::Contact,
    contact_manager::ContactManager,
    multigraph::Multigraph,
    node::Node,
    node_manager::NodeManager,
    pathfinding::Pathfinding,
    route_storage::{Guard, TreeStorage},
    types::{Date, NodeID},
};

use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use super::{schedule_multicast, schedule_unicast, RoutingOutput};

/// A structure representing the Shortest Path with Safety Nodes (SPSN) algorithm.
///
/// This struct handles routing logic and pathfinding, utilizing stored routes
/// and ensuring that the routing process adheres to specified safety and priority constraints.
///
/// # Type Parameters
/// - `NM`: A type that implements the `NodeManager` trait, responsible for managing the
///   network's nodes and their interactions.
/// - `CM`: A type that implements the `ContactManager` trait, handling contact points and
///   communication schedules within the network.
/// - `P`: A type that implements the `Pathfinding<NM, CM>` trait
pub struct Spsn<NM: NodeManager, CM: ContactManager, P: Pathfinding<NM, CM>, S: TreeStorage<NM, CM>>
{
    /// A reference-counted storage for routing data, allowing the retrieval and storage of
    /// pathfinding output.
    route_storage: Rc<RefCell<S>>,
    /// The pathfinding instance used for route calculations, responsible for computing optimal
    /// paths based on the current network state.
    pathfinding: P,
    /// The guard structure that enforces safety and priority constraints, checking if the routing
    /// can proceed based on the current bundle and its constraints.
    unicast_guard: Guard,

    // for compilation
    #[doc(hidden)]
    _phantom_nm: PhantomData<NM>,
    #[doc(hidden)]
    _phantom_cm: PhantomData<CM>,
}

impl<S: TreeStorage<NM, CM>, NM: NodeManager, CM: ContactManager, P: Pathfinding<NM, CM>>
    Spsn<NM, CM, P, S>
{
    /// Creates a new `SPSN` instance with the specified parameters.
    ///
    /// # Parameters
    ///
    /// * `nodes` - A vector of nodes representing the routing network.
    /// * `contacts` - A vector of contacts associated with the nodes.
    /// * `route_storage` - A reference-counted storage for routing data.
    /// * `with_priorities` - A boolean indicating whether to consider priorities during routing.
    ///
    /// # Returns
    ///
    /// * `Self` - A new instance of the `SPSN` struct.
    pub fn new(
        nodes: Vec<Node<NM>>,
        contacts: Vec<Contact<CM>>,
        route_storage: Rc<RefCell<S>>,
        with_priorities: bool,
    ) -> Self {
        Self {
            pathfinding: P::new(Rc::new(RefCell::new(Multigraph::new(nodes, contacts)))),
            route_storage: route_storage.clone(),
            unicast_guard: Guard::new(with_priorities),
            // for compilation
            _phantom_nm: PhantomData,
            _phantom_cm: PhantomData,
        }
    }

    /// Routes a bundle to its destination(s) using either unicast or multicast routing,
    /// depending on the number of destinations.
    ///
    /// The `route` function checks the number of destinations in `bundle`. If there is only one
    /// destination, it calls `route_unicast` to handle routing for a single target node. For multiple
    /// destinations, it calls `route_multicast` to handle routing for multiple target nodes.
    ///
    /// # Parameters
    /// - `source`: The source node ID initiating the routing operation.
    /// - `bundle`: The `Bundle` containing destination information and other relevant routing data.
    /// - `curr_time`: The current time, which affects scheduling and time-sensitive routing calculations.
    /// - `excluded_nodes`: A list of nodes to exclude from the routing paths.
    ///
    /// # Returns
    /// An `Option<RoutingOutput<CM>>`, where `Some(RoutingOutput)` contains the routing details if
    /// successful, and `None` if routing fails or encounters exclusions.
    pub fn route(
        &mut self,
        source: NodeID,
        bundle: &Bundle,
        curr_time: Date,
        excluded_nodes: &Vec<NodeID>,
    ) -> Option<RoutingOutput<CM>> {
        if bundle.destinations.len() == 1 {
            return self.route_unicast(source, bundle, curr_time, excluded_nodes);
        }

        return self.route_multicast(source, bundle, curr_time, excluded_nodes);
    }

    /// Routes a bundle to a single destination node using unicast routing.
    ///
    /// The `route_unicast` function performs a unicast routing operation for bundles with only
    /// one destination. It first checks if the unicast operation should be aborted (via `unicast_guard`).
    /// Then, it attempts to retrieve or compute a unicast tree. Finally, it schedules unicast routing
    /// using `schedule_unicast`.
    ///
    /// # Parameters
    /// - `source`: The source node ID initiating the unicast routing.
    /// - `bundle`: The `Bundle` containing the single destination and related routing data.
    /// - `curr_time`: The current time for scheduling calculations.
    /// - `excluded_nodes`: A list of nodes to exclude from the unicast path.
    ///
    /// # Returns
    /// An `Option<RoutingOutput<CM>>` containing the routing result, or `None` if routing fails or
    /// is aborted.
    fn route_unicast(
        &mut self,
        source: NodeID,
        bundle: &Bundle,
        curr_time: Date,
        excluded_nodes: &Vec<NodeID>,
    ) -> Option<RoutingOutput<CM>> {
        if self.unicast_guard.must_abort(bundle) {
            return None;
        }

        let dest = bundle.destinations[0];

        let (tree_option, _reachable_nodes) = self.route_storage.borrow().select(
            bundle,
            curr_time,
            &self.pathfinding.get_multigraph().borrow_mut().nodes,
            excluded_nodes,
        );

        if let Some(tree) = tree_option {
            return Some(schedule_unicast(
                bundle,
                curr_time,
                tree,
                &self.pathfinding.get_multigraph().borrow_mut().nodes,
                false,
            ));
        }

        let new_tree = self
            .pathfinding
            .get_next(curr_time, source, bundle, excluded_nodes);
        let tree_ref = Rc::new(RefCell::new(new_tree));

        self.route_storage
            .borrow_mut()
            .store(&bundle, tree_ref.clone());

        match &tree_ref.borrow().by_destination[dest as usize] {
            // The tree is fresh, no dry run was performed, the remained expected fail case is bundle expiration
            // Trees are not built while considering expirations for flexibility
            // /!\ But maybe it should, issues expected with non-SABR distances
            Some(route) => {
                if route.borrow().at_time > bundle.expiration {
                    return None;
                }
            }
            None => {
                self.unicast_guard.add_limit(bundle, dest as NodeID);
                return None;
            }
        }

        return Some(schedule_unicast(
            bundle,
            curr_time,
            tree_ref,
            &self.pathfinding.get_multigraph().borrow_mut().nodes,
            true,
        ));
    }

    /// Routes a bundle to multiple destination nodes using multicast routing.
    ///
    /// The `route_multicast` function performs multicast routing when `bundle` has multiple
    /// destinations. It first checks for a pre-existing multicast tree. If a tree exists and
    /// reaches all destinations, it schedules multicast routing using `schedule_multicast`.
    /// Otherwise, it creates a new multicast tree and proceeds to schedule the multicast operation.
    ///
    /// # Parameters
    /// - `source`: The source node ID initiating the multicast routing.
    /// - `bundle`: The `Bundle` containing multiple destinations.
    /// - `curr_time`: The current time for scheduling calculations.
    /// - `excluded_nodes`: A list of nodes to exclude from the multicast paths.
    ///
    /// # Returns
    /// An `Option<RoutingOutput<CM>>` containing the multicast routing result, or `None` if
    /// routing fails.
    pub fn route_multicast(
        &mut self,
        source: NodeID,
        bundle: &Bundle,
        curr_time: Date,
        excluded_nodes: &Vec<NodeID>,
    ) -> Option<RoutingOutput<CM>> {
        if let (Some(tree), Some(mut reachable_nodes)) = self.route_storage.borrow().select(
            bundle,
            curr_time,
            &self.pathfinding.get_multigraph().borrow_mut().nodes,
            excluded_nodes,
        ) {
            if bundle.destinations.len() == reachable_nodes.len() {
                return Some(schedule_multicast(
                    bundle,
                    curr_time,
                    tree,
                    &mut reachable_nodes,
                    &self.pathfinding.get_multigraph().borrow_mut().nodes,
                    false,
                ));
            }
        }

        let new_tree = self
            .pathfinding
            .get_next(curr_time, source, bundle, excluded_nodes);
        let tree = Rc::new(RefCell::new(new_tree));
        self.route_storage.borrow_mut().store(&bundle, tree.clone());

        let mut targets = Vec::new();

        return Some(schedule_multicast(
            bundle,
            curr_time,
            tree,
            &mut targets,
            &self.pathfinding.get_multigraph().borrow_mut().nodes,
            true,
        ));
    }
}
