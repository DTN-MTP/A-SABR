use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    bundle::Bundle,
    contact::Contact,
    contact_manager::ContactManager,
    node_manager::NodeManager,
    pathfinding::PathFindingOutput,
    route_stage::RouteStage,
    types::{Date, NodeID},
};

pub mod aliases;
pub mod cgr;
pub mod spsn;

/// A trait to allow generic initialization of routers.
pub trait Router<NM: NodeManager, CM: ContactManager> {
    /// Routes a bundle to its destination(s) using either unicast or multicast routing,
    /// depending on the number of destinations.
    ///
    /// The `route` function checks the number of destinations in `bundle`. If there is only one
    /// destination.
    ///
    /// # Parameters
    /// - `source`: The source node ID initiating the routing operation.
    /// - `bundle`: The `Bundle` containing destination information and other relevant routing data.
    /// - `curr_time`: The current time, which affects scheduling and time-sensitive routing calculations.
    /// - `excluded_nodes`: A list of nodes to exclude from the routing paths.
    ///
    /// # Returns
    /// An `Option<RoutingOutput<NM, CM>>`, where `Some(RoutingOutput)` contains the routing details if
    /// successful, and `None` if routing fails or encounters exclusions.
    fn route(
        &mut self,
        source: NodeID,
        bundle: &Bundle,
        curr_time: Date,
        excluded_nodes: &Vec<NodeID>,
    ) -> Option<RoutingOutput<NM, CM>>;
}

/// A struct that represents the output of a routing operation.
///
/// The `RoutingOutput` struct is used to store the results of routing calculations,
/// specifically the first hops for each destination and the associated nodes that are reachable via this the hop (e.g. for multicast).
///
/// # Fields
///
/// * `first_hops` - A hashmap mapping from a unique identifier (e.g., an index or destination ID)
///   to a tuple containing:
///     - `Rc<RefCell<Contact<NM, CM>>>`: A reference-counted, mutable reference to the `Contact`
///       that represents the first hop for the respective route.
///     - `Vec<NodeID>`: A vector of `NodeID`s representing the nodes that can be reached from
///       the first hop.
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct RoutingOutput<NM: NodeManager, CM: ContactManager> {
    pub first_hops: HashMap<
        usize,
        (
            Rc<RefCell<Contact<NM, CM>>>,
            Vec<Rc<RefCell<RouteStage<NM, CM>>>>,
        ),
    >,
}

/// Builds the routing output from the source route and reached nodes.
///
/// This function generates a `RoutingOutput` structure containing the first hops
/// for each reachable destination.
///
/// # Parameters
///
/// * `source_route` - A reference to the source route stage.
/// * `reached_nodes` - A vector of node IDs representing the nodes that were reached.
///
/// # Returns
///
/// * `RoutingOutput<NM, CM>` - The constructed routing output with first hop information.
fn build_multicast_output<NM: NodeManager, CM: ContactManager>(
    source_route: Rc<RefCell<RouteStage<NM, CM>>>,
    reached_nodes: &Vec<NodeID>,
) -> RoutingOutput<NM, CM> {
    let mut first_hops: HashMap<usize, (Rc<RefCell<Contact<NM, CM>>>, Vec<NodeID>)> =
        HashMap::new();

    for (dest, route) in source_route.borrow().next_for_destination.iter() {
        if reached_nodes.contains(dest) {
            if let Some(via) = &route.borrow().via {
                let ptr = Rc::as_ptr(&via.contact) as usize;
                if let Some((_, entry)) = first_hops.get_mut(&ptr) {
                    entry.push(*dest);
                } else {
                    first_hops.insert(ptr, (via.contact.clone(), vec![*dest]));
                }
            } else {
                panic!("Malformed route, no via contact/route!");
            }
        }
    }

    //RoutingOutput { first_hops }
    todo!()
}

/// Executes a "dry run" multicast pathfinding operation to determine the reachable destinations
/// among the multicast destinations.
///
/// `dry_run_multicast` simulates the multicast routing process for a bundle, given the current
/// network state and a pathfinding tree structure. It iterates over the destinations in the
/// bundle, checks their availability in the pathfinding tree, and initiates a recursive dry run
/// to identify reachable destinations.
///
/// # Type Parameters
/// * `NM`: A type implementing the `NodeManager` trait, which manages node-specific behaviors.
/// * `CM`: A type implementing the `ContactManager` trait, which manages contacts for routing.
/// * `D`: A type implementing the `Distance<NM, CM>` trait, defining the metric for route comparison.
///
/// # Parameters
/// * `bundle`: The `Bundle` being routed, containing the list of intended destination nodes.
/// * `at_time`: The current time at which the routing simulation is performed.
/// * `tree`: A reference-counted, mutable `PathFindingOutput<NM, CM>`, representing the
///   multicast routing tree used for pathfinding.
/// * `reachable_destinations`: A mutable vector to store `NodeID`s of destinations determined
///   to be reachable in the current run.
/// * `node_list`: A list of nodes objects.
///
/// # Returns
/// A vector of `NodeID`s representing the destinations that were successfully reached by
/// the dry run multicast operation.
pub fn dry_run_multicast<NM: NodeManager, CM: ContactManager>(
    bundle: &Bundle,
    at_time: Date,
    tree: Rc<RefCell<PathFindingOutput<NM, CM>>>,
    reachable_destinations: &mut Vec<NodeID>,
) -> Vec<NodeID> {
    let tree_ref = tree.borrow();
    for dest in &bundle.destinations {
        if let Some(_route_for_dest) = &tree_ref.by_destination[*dest as usize] {
            tree_ref.init_for_destination(*dest);
            reachable_destinations.push(*dest);
        }
    }

    let source_route = tree_ref.get_source_route();
    let mut reached_destinations: Vec<NodeID> = Vec::new();

    rec_dry_run_multicast(
        bundle,
        at_time,
        reachable_destinations,
        &mut reached_destinations,
        source_route,
        true,
    );

    return reached_destinations;
}

/// Recursively performs a dry run to determine reachable nodes.
///
/// `reachable_in_tree` is a subset of the destinations of bundle.destination.
/// `reachable_after_dry_run` is an acc subset of reachable_in_tree and the expected output.
///
/// # Parameters
///
/// * `bundle` - The current bundle containing routing information.
/// * `at_time` - The current date/time for the routing operation.
/// * `reachable_in_tree` - The nodes that are reachable within the tree.
/// * `reachable_after_dry_run` - A mutable vector to accumulate reachable nodes.
/// * `route` - The current route stage being evaluated.
/// * `is_source` - A boolean indicating if the route is the source route.
/// * `node_list`: A list of nodes objects.
fn rec_dry_run_multicast<NM: NodeManager, CM: ContactManager>(
    bundle: &Bundle,
    mut at_time: Date,
    reachable_in_tree: &Vec<NodeID>,
    reachable_after_dry_run: &mut Vec<NodeID>,
    route: Rc<RefCell<RouteStage<NM, CM>>>,
    is_source: bool,
) {
    let mut route_borrowed = route.borrow_mut();

    #[cfg(feature = "node_proc")]
    let bundle_to_consider = route_borrowed.bundle.clone();
    #[cfg(not(feature = "node_proc"))]
    let bundle_to_consider = bundle;

    if !is_source {
        if !route_borrowed.dry_run(at_time, &bundle_to_consider, false) {
            return;
        }
        at_time = route_borrowed.at_time;
    }

    // use the ptr pointed by the rc (as usize) as key, TODO: fix this ugly workaround
    let mut next_routes: HashMap<usize, (Rc<RefCell<RouteStage<NM, CM>>>, Vec<NodeID>)> =
        HashMap::new();
    for dest in reachable_in_tree {
        if route_borrowed.to_node == *dest {
            reachable_after_dry_run.push(*dest);
        } else if let Some(next_route) = route_borrowed.next_for_destination.get(&dest) {
            let ptr = Rc::as_ptr(next_route) as usize;
            if let Some((_, entry)) = next_routes.get_mut(&ptr) {
                entry.push(*dest);
            } else {
                next_routes.insert(ptr, (next_route.clone(), vec![*dest]));
            }
        }
    }
    for (_, (next_route, destinations)) in next_routes.into_iter() {
        rec_dry_run_multicast(
            &bundle_to_consider,
            at_time,
            &destinations,
            reachable_after_dry_run,
            next_route.clone(),
            false,
        );
    }
}

/// Recursively updates routes based on scheduled contacts.
///
/// # Parameters
///
/// * `bundle` - The current bundle containing routing information.
/// * `at_time` - The current date/time for the routing operation.
/// * `reachable_after_dry_run` - The nodes that were reachable after the dry run.
/// * `route` - The current route stage being updated.
/// * `is_source` - A boolean indicating if the route is the source route.
/// * `node_list`: A list of nodes objects.
fn rec_update_multicast<NM: NodeManager, CM: ContactManager>(
    bundle: &Bundle,
    mut at_time: Date,
    reachable_after_dry_run: &Vec<NodeID>,
    route: Rc<RefCell<RouteStage<NM, CM>>>,
    is_source: bool,
) {
    let mut route_borrowed = route.borrow_mut();

    #[cfg(feature = "node_proc")]
    let bundle_to_consider = route_borrowed.bundle.clone();
    #[cfg(not(feature = "node_proc"))]
    let bundle_to_consider = bundle;

    if !is_source {
        if !route_borrowed.schedule(at_time, &bundle_to_consider) {
            return;
        }
        at_time = route_borrowed.at_time;
    }

    // use the ptr pointed by the rc (as usize) as key, TODO: fix this ugly workaround
    let mut next_routes: HashMap<usize, (Rc<RefCell<RouteStage<NM, CM>>>, Vec<NodeID>)> =
        HashMap::new();
    for dest in reachable_after_dry_run {
        if route_borrowed.to_node == *dest {
            continue;
        } else if let Some(next_route) = route_borrowed.next_for_destination.get(dest) {
            let ptr = Rc::as_ptr(next_route) as usize;
            if let Some((_, entry)) = next_routes.get_mut(&ptr) {
                entry.push(*dest);
            } else {
                next_routes.insert(ptr, (next_route.clone(), vec![*dest]));
            }
        }
    }

    for (_, (next_route, destinations)) in next_routes.into_iter() {
        rec_update_multicast(
            &bundle_to_consider,
            at_time,
            &destinations,
            next_route.clone(),
            false,
        );
    }
}

/// Schedules routing operations based on the source node and a multicast bundle.
///
/// This function determines reachable destinations, executes a dry run,
/// updates the routes based on the dry run results, and prepares the output.
///
/// # Parameters
///
/// * `source` - The ID of the source node initiating the route.
/// * `bundle` - The current bundle containing routing information.
/// * `curr_time` - The current date/time for the routing operation.
/// * `tree_ref` - A reference to the pathfinding output.
/// * `dry_run_to_fill_targets` - Set this boolean to true if the tree is fresh (i.e. the dry run
/// from selection did not occur).
///
/// # Returns
///
/// * `RoutingOutput<NM, CM>` - The routing output.
fn schedule_multicast<NM: NodeManager, CM: ContactManager>(
    bundle: &Bundle,
    curr_time: Date,
    tree: Rc<RefCell<PathFindingOutput<NM, CM>>>,
    targets: &mut Vec<NodeID>,
    dry_run_to_fill_targets: bool,
) -> RoutingOutput<NM, CM> {
    if dry_run_to_fill_targets {
        *targets = dry_run_multicast(bundle, curr_time, tree.clone(), targets);
    }

    let source_route = tree.borrow().get_source_route();

    rec_update_multicast(bundle, curr_time, targets, source_route.clone(), true);

    return build_multicast_output(source_route, targets);
}

/// Macro to create customized unicast `dry_run` pathfinding functions with flexible routing behavior.
///
/// `create_dry_run_unicast_path_variant` generates a unicast pathfinding function that supports
/// both optional exclusion filtering and optional route initialization. This is especially useful
/// for adapting the pathfinding process to different routing scenarios.
///
/// - **Exclusions**: Some routing protocols require excluding specific nodes from pathfinding,
///   at the selection stage (e.g. CGR) while node exclusion can also occur at tree construction
///   (e.g. SPSN). This macro allows conditional exclusion handling by using the `$apply_exclusions`
///   parameter.
/// - **Initialization**: In certain cases, the destination route may need initialization at the
///   beginning of pathfinding. The `$try_init` parameter controls whether this initialization
///   step is performed. E.g. SPSN do not initialize the routes for each destination of the tree,
///   while CGR would init any path before being sent to storage.
///
/// # Parameters
/// - `$fn_name`: The name of the generated function, allowing multiple pathfinding function
///   variants to be created for different protocols or exclusion behaviors.
/// - `$apply_exclusions`: A boolean flag to control whether exclusion handling is enabled in the
///   generated function.
/// - `$try_init`: A boolean flag to specify if the destination route should be initialized at
///   the beginning of the function.
macro_rules! create_dry_run_unicast_path_variant {
    ($fn_name:ident, $apply_exclusions:ident, $try_init:ident) => {
        /// Generated by macro.
        ///
        /// # Parameters
        /// - `bundle`: The `Bundle` being routed, containing the destination node(s).
        /// - `at_time`: The starting time for the dry run pathfinding.
        /// - `source_route`: The starting `RouteStage` of the route.
        /// - `dest_route`: The target `RouteStage` of the route.
        /// - `node_list`: A list of nodes (`Node<NM>`) in the network.
        /// # Returns
        /// The function will return an `Option` containing the final `RouteStage` if a route to the
        /// destination was found, or `None` if the pathfinding failed.
        pub fn $fn_name<NM: NodeManager, CM: ContactManager>(
            bundle: &Bundle,
            mut at_time: Date,
            source_route: Rc<RefCell<RouteStage<NM, CM>>>,
            dest_route: Rc<RefCell<RouteStage<NM, CM>>>,
        ) -> Option<Rc<RefCell<RouteStage<NM, CM>>>> {
            let dest = bundle.destinations[0];

            if $try_init {
                RouteStage::init_route(dest_route);
            }

            let mut curr_opt = source_route
                .borrow()
                .next_for_destination
                .get(&dest)
                .cloned();

            while let Some(curr_route) = curr_opt {
                let mut curr_route_borrowed = curr_route.borrow_mut();

                #[cfg(feature = "node_proc")]
                let bundle_to_consider = curr_route_borrowed.bundle.clone();
                #[cfg(not(feature = "node_proc"))]
                let bundle_to_consider = bundle;

                if !curr_route_borrowed.dry_run(at_time, &bundle_to_consider, false) {
                    return None;
                }

                at_time = curr_route_borrowed.at_time;

                if curr_route_borrowed.to_node == dest {
                    return Some(curr_route.clone());
                }

                curr_opt = curr_route_borrowed.next_for_destination.get(&dest).cloned();
            }

            None
        }
    };
}

create_dry_run_unicast_path_variant!(dry_run_unicast_path, false, true);
create_dry_run_unicast_path_variant!(dry_run_unicast_path_with_exclusions, true, false);

/// Executes a dry run of unicast pathfinding within a multicast tree structure.
///
/// `dry_run_unicast_tree` performs unicast pathfinding for a given `bundle`, starting from the
/// tree's source route and attempting to reach the specified destination node. The function
/// searches the multicast tree to find a viable path to the destination. If the path is found,
/// it uses the unicast pathfinding function `dry_run_unicast_path` to finalize the route.
///
/// # Parameters
/// - `bundle`: The `Bundle` to be routed, containing destination nodes.
/// - `at_time`: The starting time for the dry run pathfinding.
/// - `tree`: An `Rc<RefCell<PathFindingOutput<NM, CM>>>` containing the multicast tree structure
///   with route stages mapped by destination.
/// - `node_list`: A list of nodes (`Node<NM>`) in the network, used in the pathfinding process.
///
/// # Returns
/// Returns an `Option<Rc<RefCell<RouteStage<NM, CM>>>>` containing the route stage to the
/// destination if a valid path is found, or `None` if no path is available.
pub fn dry_run_unicast_tree<NM: NodeManager, CM: ContactManager>(
    bundle: &Bundle,
    at_time: Date,
    tree: Rc<RefCell<PathFindingOutput<NM, CM>>>,
) -> Option<Rc<RefCell<RouteStage<NM, CM>>>> {
    let dest = bundle.destinations[0];
    let tree_ref = tree.borrow();
    if tree_ref.by_destination[dest as usize].is_none() {
        return None;
    }
    let source_route = tree_ref.get_source_route();
    if let Some(dest_route) = tree_ref.by_destination[dest as usize].clone() {
        return dry_run_unicast_path(bundle, at_time, source_route, dest_route);
    }
    None
}

/// Iteratively updates routes based on scheduled contacts.
///
/// # Parameters
///
/// * `bundle` - The current bundle containing routing information.
/// * `dest` - The destination for the bundle.
/// * `at_time` - The current date/time for the routing operation.
/// * `source_route` - The source route.
fn update_unicast<NM: NodeManager, CM: ContactManager>(
    bundle: &Bundle,
    dest: NodeID,
    mut at_time: Date,
    source_route: Rc<RefCell<RouteStage<NM, CM>>>,
) -> RoutingOutput<NM, CM> {
    let mut curr_opt = source_route
        .borrow()
        .next_for_destination
        .get(&dest)
        .cloned();

    let mut first_hop: Option<Rc<RefCell<Contact<NM, CM>>>> = None;

    while let Some(curr_route) = curr_opt {
        let mut curr_route_borrowed = curr_route.borrow_mut();

        if first_hop.is_none() {
            first_hop = curr_route_borrowed.get_via_contact();
        }

        #[cfg(feature = "node_proc")]
        let bundle_to_consider = curr_route_borrowed.bundle.clone();
        #[cfg(not(feature = "node_proc"))]
        let bundle_to_consider = bundle;

        if !curr_route_borrowed.schedule(at_time, &bundle_to_consider) {
            panic!("Faulty dry run, didn't allow a clean update!");
        }

        at_time = curr_route_borrowed.at_time;

        if curr_route_borrowed.to_node == dest {
            if let Some(first) = first_hop {
                let mut first_hops: HashMap<
                    usize,
                    (
                        Rc<RefCell<Contact<NM, CM>>>,
                        Vec<Rc<RefCell<RouteStage<NM, CM>>>>,
                    ),
                > = HashMap::new();
                first_hops.insert(first.as_ptr() as usize, (first, vec![curr_route.clone()]));
                return RoutingOutput { first_hops };
            }
            panic!("First hop tracking issue");
        }

        curr_opt = curr_route_borrowed.next_for_destination.get(&dest).cloned();
    }

    panic!("Faulty dry run, didn't allow a clean update!");
}

/// Schedules a unicast routing operation, optionally initializing the multicast tree.
///
/// The `schedule_unicast` function schedules a unicast pathfinding operation for the provided
/// `bundle`, which targets a specified destination node within the multicast tree. If
/// `init_tree` is `true`, it initializes the tree for routing to the destination. Then, it
/// updates the unicast route using `update_unicast` and finalizes the routing output via
/// `build_unicast_output`.
///
/// # Parameters
/// - `bundle`: The `Bundle` to route, containing the destination node(s).
/// - `curr_time`: The current time, used as the starting time for scheduling.
/// - `tree`: An `Rc<RefCell<PathFindingOutput<NM, CM>>>`, representing the multicast tree structure,
///   which holds route stages by destination.
/// - `node_list`: A list of nodes (`Node<NM>`) in the network.
/// - `init_tree`: A boolean flag indicating whether to initialize the tree for routing to the
///   destination node.
///
/// # Returns
/// Returns a `RoutingOutput<NM, CM>` containing the scheduled routing details.
fn schedule_unicast<NM: NodeManager, CM: ContactManager>(
    bundle: &Bundle,
    curr_time: Date,
    tree: Rc<RefCell<PathFindingOutput<NM, CM>>>,
    init_tree: bool,
) -> RoutingOutput<NM, CM> {
    if init_tree {
        tree.borrow().init_for_destination(bundle.destinations[0]);
    }

    let dest = bundle.destinations[0];
    let source_route = tree.borrow().get_source_route();
    return update_unicast(bundle, dest, curr_time, source_route.clone());
}

/// Schedules a unicast pathfinding operation for a given source route without tree initialization.
///
/// The `schedule_unicast_path` function is similar to `schedule_unicast` but skips tree
/// initialization. Instead, it directly performs unicast pathfinding starting from the specified
/// `source_route` and uses `update_unicast` to compute the route. Finally, it generates the
/// routing output using `build_unicast_output`.
///
/// # Parameters
/// - `bundle`: The `Bundle` to route, containing the destination node(s).
/// - `curr_time`: The current time, used as the starting time for scheduling.
/// - `source_route`: The starting `RouteStage` for unicast pathfinding.
/// - `node_list`: A list of nodes (`Node<NM>`) in the network.
///
/// # Returns
/// Returns a `RoutingOutput<NM, CM>` containing the scheduled routing details.
fn schedule_unicast_path<NM: NodeManager, CM: ContactManager>(
    bundle: &Bundle,
    curr_time: Date,
    source_route: Rc<RefCell<RouteStage<NM, CM>>>,
) -> RoutingOutput<NM, CM> {
    let dest = bundle.destinations[0];
    return update_unicast(bundle, dest, curr_time, source_route.clone());
}
