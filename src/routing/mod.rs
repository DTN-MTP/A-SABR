use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    bundle::Bundle,
    contact::Contact,
    contact_manager::ContactManager,
    errors::ASABRError,
    node_manager::NodeManager,
    pathfinding::PathFindingOutput,
    route_stage::{RouteStage, SharedRouteStage},
    types::{Date, NodeID},
};

pub mod aliases;
pub mod cgr;
pub mod spsn;
pub mod volcgr;

type FirstHopsVec<NM, CM> = (
    Rc<RefCell<Contact<NM, CM>>>,
    Vec<Rc<RefCell<RouteStage<NM, CM>>>>,
);
type FirstHop<NM, CM> = (
    Rc<RefCell<Contact<NM, CM>>>,
    Rc<RefCell<RouteStage<NM, CM>>>,
);

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
        excluded_nodes: &[NodeID],
    ) -> Result<Option<RoutingOutput<NM, CM>>, ASABRError>;
}

/// A struct that represents the output of a routing operation.
///
/// The `RoutingOutput` struct is used to store the results of routing calculations,
/// specifically the first hops for each destination and the associated nodes that are reachable via this the hop (e.g. for multicast).
///
/// # Fields
///
/// * `first_hops` - A hashmap mapping from a unique identifier (e.g., an index or destination ID)
///   to a `FirstHopsVec` tuple containing:
///     - `Rc<RefCell<Contact<NM, CM>>>`: A reference-counted, mutable reference to the `Contact`
///       that represents the first hop for the respective route.
///     - `Vec<Rc<RefCell<RouteStage<NM, CM>>>>`: A vector of reference-counted, mutable
///       references to `RouteStage`s to the nodes that can be reached from the first hop.
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct RoutingOutput<NM: NodeManager, CM: ContactManager> {
    pub first_hops: HashMap<usize, FirstHopsVec<NM, CM>>,
}

impl<NM: NodeManager, CM: ContactManager> RoutingOutput<NM, CM> {
    pub fn lazy_get_for_unicast(&self, dest: NodeID) -> Option<FirstHop<NM, CM>> {
        for (contact, dest_routes) in self.first_hops.values() {
            for route_rc in dest_routes {
                if route_rc.borrow().to_node == dest {
                    return Some((contact.clone(), route_rc.clone()));
                }
            }
        }
        None
    }
}

pub fn dry_run_multicast<NM: NodeManager, CM: ContactManager>(
    bundle: &Bundle,
    at_time: Date,
    tree: Rc<RefCell<PathFindingOutput<NM, CM>>>,
) -> Result<Vec<NodeID>, ASABRError> {
    let tree_ref = tree.borrow();
    let mut dests_in_tree = Vec::new();
    let mut reached_destinations = Vec::new();
    for dest in &bundle.destinations {
        if let Some(_route_for_dest) = &tree_ref.by_destination[*dest as usize] {
            tree_ref.init_for_destination(*dest)?;
            dests_in_tree.push(*dest);
        }
    }

    let source_route = tree_ref.get_source_route();
    let mut accumulator = vec![(source_route, true, at_time, dests_in_tree)];
    #[cfg(not(feature = "node_proc"))]
    let bundle_to_consider = bundle;

    while let Some((current_route, is_source, mut time, downstream_dests)) = accumulator.pop() {
        let mut route_borrowed = current_route.try_borrow_mut()?;

        #[cfg(feature = "node_proc")]
        let bundle_to_consider = route_borrowed.bundle.clone();

        if !is_source {
            if !route_borrowed.dry_run(time, &bundle_to_consider, false)? {
                continue;
            }
            time = route_borrowed.at_time;
        }

        let reached_node = route_borrowed.to_node;

        let mut next_routes: HashMap<usize, (SharedRouteStage<NM, CM>, Vec<NodeID>)> =
            HashMap::new();
        for dest in downstream_dests {
            if reached_node == dest {
                reached_destinations.push(dest);
            } else if let Some(next_route) = route_borrowed.next_for_destination.get(&dest) {
                let ptr = Rc::as_ptr(next_route) as usize;
                if let Some((_, entry)) = next_routes.get_mut(&ptr) {
                    entry.push(dest);
                } else {
                    next_routes.insert(ptr, (next_route.clone(), vec![dest]));
                }
            }
        }
        for (_ptr, (next_route, next_downstream_dests)) in next_routes {
            accumulator.push((next_route, false, time, next_downstream_dests));
        }
    }

    Ok(reached_destinations)
}

fn update_multicast<NM: NodeManager, CM: ContactManager>(
    _bundle: &Bundle,
    at_time: Date,
    reachable_after_dry_run: Vec<NodeID>,
    source_route: SharedRouteStage<NM, CM>,
) -> Result<RoutingOutput<NM, CM>, ASABRError> {
    let mut first_hops_map: HashMap<usize, FirstHopsVec<NM, CM>> = HashMap::new();
    let mut accumulator: Vec<(SharedRouteStage<NM, CM>, Option<usize>, Date, Vec<u16>)> =
        vec![(source_route, None, at_time, reachable_after_dry_run)];
    #[cfg(not(feature = "node_proc"))]
    let bundle_to_consider = _bundle;

    while let Some((current_route, mut first_hop_ptr, mut time, downstream_dests)) =
        accumulator.pop()
    {
        let mut route_borrowed = current_route.try_borrow_mut()?;

        #[cfg(feature = "node_proc")]
        let bundle_to_consider = route_borrowed.bundle.clone();

        if first_hop_ptr.is_some() {
            if route_borrowed.schedule(time, &bundle_to_consider).is_err() {
                continue;
            }
            time = route_borrowed.at_time;
        }
        let reached_node = route_borrowed.to_node;

        let mut next_routes: HashMap<usize, (SharedRouteStage<NM, CM>, Vec<NodeID>)> =
            HashMap::new();
        for dest in downstream_dests {
            if reached_node == dest {
                let Some(ptr) = first_hop_ptr else { continue };
                let Some((_, rts)) = first_hops_map.get_mut(&ptr) else {
                    continue;
                };
                rts.push(current_route.clone());
            } else if let Some(next_route) = route_borrowed.next_for_destination.get(&dest) {
                let ptr = Rc::as_ptr(next_route) as usize;
                if let Some((_, entry)) = next_routes.get_mut(&ptr) {
                    entry.push(dest);
                } else {
                    next_routes.insert(ptr, (next_route.clone(), vec![dest]));
                }
            }
        }
        for (_ptr, (next_route, next_downstream_dests)) in next_routes {
            if first_hop_ptr.is_none() {
                let first_hop_contact = next_route.borrow().get_via_contact();
                if let Some(first_hop_contact) = first_hop_contact {
                    let ptr = first_hop_contact.as_ptr() as usize;
                    first_hop_ptr = Some(ptr);
                    first_hops_map
                        .entry(ptr)
                        .or_insert_with(|| (first_hop_contact, Vec::new()));
                }
            }
            accumulator.push((next_route, first_hop_ptr, time, next_downstream_dests));
        }
    }
    Ok(RoutingOutput {
        first_hops: first_hops_map,
    })
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
///   from selection did not occur).
///
/// # Returns
///
/// * `RoutingOutput<NM, CM>` - The routing output.
fn schedule_multicast<NM: NodeManager, CM: ContactManager>(
    bundle: &Bundle,
    curr_time: Date,
    tree: Rc<RefCell<PathFindingOutput<NM, CM>>>,
    targets_opt: Option<Vec<NodeID>>,
) -> Result<RoutingOutput<NM, CM>, ASABRError> {
    let targets = match targets_opt {
        Some(targets) => targets,
        None => dry_run_multicast(bundle, curr_time, tree.clone())?,
    };
    let source_route = tree.borrow().get_source_route();
    update_multicast(bundle, curr_time, targets, source_route.clone())
}

pub fn dry_run_unicast_path<NM: NodeManager, CM: ContactManager>(
    bundle: &Bundle,
    mut at_time: Date,
    source_route: SharedRouteStage<NM, CM>,
    with_exclusions: bool,
) -> Result<Option<SharedRouteStage<NM, CM>>, ASABRError> {
    let dest = bundle.destinations[0];

    let mut curr_opt = source_route
        .borrow()
        .next_for_destination
        .get(&dest)
        .cloned();
    #[cfg(not(feature = "node_proc"))]
    let bundle_to_consider = bundle;
    while let Some(curr_route) = curr_opt {
        let mut curr_route_borrowed = curr_route.try_borrow_mut()?;

        #[cfg(feature = "node_proc")]
        let bundle_to_consider = curr_route_borrowed.bundle.clone();

        if !curr_route_borrowed.dry_run(at_time, &bundle_to_consider, with_exclusions)? {
            return Ok(None);
        }

        at_time = curr_route_borrowed.at_time;

        if curr_route_borrowed.to_node == dest {
            return Ok(Some(curr_route.clone()));
        }

        curr_opt = curr_route_borrowed.next_for_destination.get(&dest).cloned();
    }

    Ok(None)
}

// ($fn_name:ident, $apply_exclusions:ident, $try_init:ident)
// create_dry_run_unicast_path_variant!(dry_run_unicast_path, false, true);
// create_dry_run_unicast_path_variant!(dry_run_unicast_path_with_exclusions, true, false);

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
/// Returns an `Option<SharedRouteStage<NM, CM>>` containing the route stage to the
/// destination if a valid path is found, or `None` if no path is available.
pub fn dry_run_unicast_tree<NM: NodeManager, CM: ContactManager>(
    bundle: &Bundle,
    at_time: Date,
    tree: Rc<RefCell<PathFindingOutput<NM, CM>>>,
    with_exclusions: bool,
) -> Result<Option<SharedRouteStage<NM, CM>>, ASABRError> {
    let dest = bundle.destinations[0];
    let tree_ref = tree.borrow();
    let dest_route_req = tree_ref.by_destination.get(dest as usize).cloned();

    let Some(dest_route_opt) = dest_route_req else {
        return Err(ASABRError::DryRunError("Unknown destination"));
    };

    let Some(dest_route) = dest_route_opt else {
        return Ok(None);
    };

    let source_route = tree_ref.get_source_route();

    RouteStage::init_route(dest_route)?;
    dry_run_unicast_path(bundle, at_time, source_route, with_exclusions)
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
    _bundle: &Bundle,
    dest: NodeID,
    mut at_time: Date,
    source_route: SharedRouteStage<NM, CM>,
) -> Result<RoutingOutput<NM, CM>, ASABRError> {
    if source_route.borrow().to_node == dest {
        return Err(ASABRError::ScheduleError(
            "Bundle's destination is equal to source",
        ));
    }

    let mut curr_opt = source_route
        .borrow()
        .next_for_destination
        .get(&dest)
        .cloned();

    let mut first_hop: Option<Rc<RefCell<Contact<NM, CM>>>> = None;
    #[cfg(not(feature = "node_proc"))]
    let bundle_to_consider = _bundle;
    while let Some(curr_route) = curr_opt {
        let mut curr_route_borrowed = curr_route.try_borrow_mut()?;

        if first_hop.is_none() {
            first_hop = curr_route_borrowed.get_via_contact();
        }

        #[cfg(feature = "node_proc")]
        let bundle_to_consider = curr_route_borrowed.bundle.clone();

        curr_route_borrowed.schedule(at_time, &bundle_to_consider)?;

        at_time = curr_route_borrowed.at_time;

        if curr_route_borrowed.to_node == dest {
            if let Some(first) = first_hop {
                let mut first_hops: HashMap<usize, FirstHopsVec<NM, CM>> = HashMap::new();
                first_hops.insert(first.as_ptr() as usize, (first, vec![curr_route.clone()]));
                return Ok(RoutingOutput { first_hops });
            }
            return Err(ASABRError::ScheduleError("First hop tracking issue"));
        }

        curr_opt = curr_route_borrowed.next_for_destination.get(&dest).cloned();
    }

    Err(ASABRError::ScheduleError("Faulty dry run"))
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
) -> Result<RoutingOutput<NM, CM>, ASABRError> {
    if init_tree {
        tree.borrow().init_for_destination(bundle.destinations[0])?;
    }
    let dest = bundle.destinations[0];
    let source_route = tree.borrow().get_source_route();
    update_unicast(bundle, dest, curr_time, source_route.clone())
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
    source_route: SharedRouteStage<NM, CM>,
) -> Result<RoutingOutput<NM, CM>, ASABRError> {
    let dest = bundle.destinations[0];
    update_unicast(bundle, dest, curr_time, source_route.clone())
}
