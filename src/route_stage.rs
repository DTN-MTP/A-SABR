use crate::bundle::Bundle;
use crate::contact::Contact;
use crate::contact_manager::ContactManager;
use crate::node::Node;
use crate::node_manager::NodeManager;
use crate::types::{Date, Duration, HopCount, NodeID};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Represents an intermediate hop in a route, typically used for multi-hop communication or routing.
///
/// This struct encapsulates the `Contact` and parent `RouteStage` information necessary to move from
/// one stage to the next.
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct ViaHop<CM: ContactManager> {
    /// A reference to the contact for this hop, representing the intermediate node.
    pub contact: Rc<RefCell<Contact<CM>>>,
    /// A reference to the parent route stage for this hop.
    pub parent_route: Rc<RefCell<RouteStage<CM>>>,
}

impl<CM: ContactManager> Clone for ViaHop<CM> {
    fn clone(&self) -> Self {
        ViaHop {
            contact: Rc::clone(&self.contact),
            parent_route: Rc::clone(&self.parent_route),
        }
    }
}

/// Represents a stage in the routing process to a destination node.
///
///  # Type Parameters
/// - `CM`: A type implementing the `ContactManager` trait, responsible for managing the
///   contact's operations.
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct RouteStage<CM: ContactManager> {
    /// The ID of the destination node for this route stage.
    pub to_node: NodeID,
    /// The time at which this route stage is considered to be valid or relevant.
    pub at_time: Date,
    /// A flag that indicates if this stage of the route is disabled.
    pub is_disabled: bool,
    /// An optional `ViaHop` that stores information about the intermediate hops that lead to this stage.
    pub via: Option<ViaHop<CM>>,
    /// The number of hops taken to reach this stage from the source.
    pub hop_count: HopCount,
    /// The cumulative delay incurred on the path to this stage, often used for routing optimizations.
    pub cumulative_delay: Duration,
    /// The time at which this route stage expires, indicating when it is no longer valid.
    pub expiration: Date,
    /// A flag indicating whether the route has been fully initialized and is ready for routing.
    pub route_initialized: bool,
    /// A hashmap that maps destination node IDs to their respective next route stages.
    pub next_for_destination: HashMap<NodeID, Rc<RefCell<RouteStage<CM>>>>,

    #[cfg(feature = "bundle_processing")]
    /// The stage of the bundle that arrives at to_node
    pub bundle_opt: Bundle,
}

impl<CM: ContactManager> RouteStage<CM> {
    /// Creates a new `RouteStage` with the specified parameters.
    ///
    /// # Parameters
    ///
    /// * `at_time` - The time at which this route stage is scheduled.
    /// * `to_node` - The destination node ID.
    /// * `via_hop` - An optional ViaHop information.
    ///
    /// # Returns
    ///
    /// A new instance of `RouteStage`.

    pub fn new(
        at_time: Date,
        to_node: NodeID,
        via_hop: Option<ViaHop<CM>>,
        #[cfg(feature = "bundle_processing")] bundle: Bundle,
    ) -> Self {
        Self {
            to_node,
            at_time,
            is_disabled: false,
            via: via_hop,
            hop_count: 0,
            cumulative_delay: 0.0,
            expiration: Date::MAX,
            route_initialized: false,
            next_for_destination: HashMap::new(),
            #[cfg(feature = "bundle_processing")]
            bundle_opt: bundle,
        }
    }

    pub fn clone(&self) -> RouteStage<CM> {
        let mut route = Self::new(
            self.at_time,
            self.to_node,
            self.via.clone(),
            #[cfg(feature = "bundle_processing")]
            self.bundle_opt.clone(),
        );
        route.is_disabled = self.is_disabled;
        route.via = self.via.clone();
        route.hop_count = self.hop_count;
        route.cumulative_delay = self.cumulative_delay;
        route.expiration = self.expiration;

        return route;
    }

    pub fn init_route(route: Rc<RefCell<RouteStage<CM>>>) {
        let destination = route.borrow().to_node;
        {
            if route.borrow().route_initialized {
                return;
            }
        }

        let mut curr_opt: Option<Rc<RefCell<RouteStage<CM>>>> = Some(route.clone());

        while let Some(current) = curr_opt.take() {
            let route_borrowed = current.borrow_mut();
            if let Some(ref parent) = route_borrowed.via {
                parent
                    .parent_route
                    .borrow_mut()
                    .next_for_destination
                    .insert(destination, current.clone());
                curr_opt = Some(Rc::clone(&parent.parent_route));
            }
        }

        route.borrow_mut().route_initialized = true;
    }

    /// Schedules the transmission of a `bundle` through a network using the provided node list.
    ///
    /// This function schedules the transmission by interacting with the contact manager and the nodes
    /// in the `node_list`. If node management is enabled (not behind the `enable_node_management` feature flag),
    /// the nodes will be queried for their transmission and reception schedules. The function will return `true`
    /// if the scheduling is successful and the bundle is scheduled, or `false` if any failure occurs.
    ///
    /// # Arguments
    ///
    /// * `at_time` - current time at the tx node.
    /// * `bundle` - The bundle to be transmitted.
    /// * `node_list` - A reference to the list of nodes where transmission and reception occur.
    ///
    /// # Returns
    ///
    /// * `true` if the scheduling process was successful and the bundle is properly scheduled.
    /// * `false` if the scheduling process failed for any reason, such as a node being excluded, timing constraints, or invalid transmission conditions.
    pub fn schedule<NM: NodeManager>(
        &mut self,
        at_time: Date,
        bundle: &Bundle,
        node_list: &Vec<Rc<RefCell<Node<NM>>>>,
    ) -> bool {
        if let Some(via) = &self.via {
            let mut contact_borrowed = via.contact.borrow_mut();
            let info = contact_borrowed.info;

            // If bundle processing is enabled, a mutable bundle copy is required to be attached to the RouteStage.
            #[cfg(feature = "bundle_processing")]
            let mut bundle_to_consider = bundle.clone();
            #[cfg(not(feature = "bundle_processing"))]
            let bundle_to_consider = bundle;

            #[cfg(feature = "enable_node_management")]
            let mut tx_node = node_list[contact_borrowed.get_tx_node() as usize].borrow_mut();
            #[cfg(feature = "enable_node_management")]
            let mut rx_node = node_list[contact_borrowed.get_rx_node() as usize].borrow_mut();

            #[cfg(feature = "bundle_processing")]
            let sending_time = tx_node
                .manager
                .schedule_process(at_time, &mut bundle_to_consider);
            #[cfg(not(feature = "bundle_processing"))]
            let sending_time = at_time;

            if let Some(res) =
                contact_borrowed
                    .manager
                    .schedule(&info, sending_time, &bundle_to_consider)
            {
                #[cfg(feature = "enable_node_management")]
                if !tx_node
                    .manager
                    .schedule_tx(res.tx_start, res.tx_end, &bundle_to_consider)
                {
                    return false;
                }

                let arrival_time = res.tx_end + res.delay;

                if arrival_time > bundle_to_consider.expiration {
                    return false;
                }
                #[cfg(feature = "enable_node_management")]
                if !rx_node.manager.schedule_rx(
                    res.tx_start + res.delay,
                    res.tx_end + res.delay,
                    &bundle_to_consider,
                ) {
                    return false;
                }

                self.at_time = arrival_time;
                #[cfg(feature = "bundle_processing")]
                {
                    self.bundle_opt = bundle_to_consider;
                }
                return true;
            }
        }
        return false;
    }

    /// Performs a dry run to simulate the transmission of a `bundle` through a network without actually
    /// scheduling it. This function checks if the transmission can occur, considering factors such as exclusions
    /// and timing constraints, but does not perform any actual node scheduling or updates.
    ///
    /// If node management is enabled, the nodes will be simulated to check whether the transmission and reception
    /// schedules are valid. The `with_exclusions` flag can be used to check whether the receiving node is excluded
    /// from the transmission.
    ///
    /// # Arguments
    ///
    /// * `at_time` - current time at the tx node.
    /// * `bundle` - The bundle to simulate transmission for.
    /// * `node_list` - A reference to the list of nodes where transmission and reception occur.
    /// * `with_exclusions` - If `true`, checks whether the receiving node is excluded from the transmission. If `false`, no exclusions are checked.
    ///
    /// # Returns
    ///
    /// * `true` if the dry run was successful and the bundle can be transmitted according to the simulation.
    /// * `false` if the dry run fails, such as due to an excluded node, invalid timing, or any other condition preventing transmission.
    pub fn dry_run<NM: NodeManager>(
        &mut self,
        at_time: Date,
        bundle: &Bundle,
        node_list: &Vec<Rc<RefCell<Node<NM>>>>,
        with_exclusions: bool,
    ) -> bool {
        if let Some(via) = &self.via {
            let contact_borrowed = via.contact.borrow_mut();
            let info = contact_borrowed.info;

            if with_exclusions {
                {
                    let node = node_list[contact_borrowed.get_rx_node() as usize].borrow();
                    if node.info.excluded {
                        return false;
                    }
                }
            }

            // If bundle processing is enabled, a mutable bundle copy is required to be attached to the RouteStage.
            #[cfg(feature = "bundle_processing")]
            let mut bundle_to_consider = bundle.clone();
            #[cfg(not(feature = "bundle_processing"))]
            let bundle_to_consider = bundle;

            #[cfg(feature = "enable_node_management")]
            let mut tx_node = node_list[contact_borrowed.get_tx_node() as usize].borrow_mut();
            #[cfg(feature = "enable_node_management")]
            let mut rx_node = node_list[contact_borrowed.get_rx_node() as usize].borrow_mut();

            #[cfg(feature = "bundle_processing")]
            let sending_time = tx_node
                .manager
                .dry_run_process(at_time, &mut bundle_to_consider);

            #[cfg(not(feature = "bundle_processing"))]
            let sending_time = at_time;

            if let Some(res) =
                contact_borrowed
                    .manager
                    .dry_run(&info, sending_time, &bundle_to_consider)
            {
                #[cfg(feature = "enable_node_management")]
                if !tx_node
                    .manager
                    .dry_run_tx(res.tx_start, res.tx_end, &bundle_to_consider)
                {
                    return false;
                }

                let arrival_time = res.tx_end + res.delay;

                if arrival_time > bundle_to_consider.expiration {
                    return false;
                }
                #[cfg(feature = "enable_node_management")]
                if !rx_node.manager.dry_run_rx(
                    res.tx_start + res.delay,
                    res.tx_end + res.delay,
                    &bundle_to_consider,
                ) {
                    return false;
                }

                self.at_time = arrival_time;
                #[cfg(feature = "bundle_processing")]
                {
                    self.bundle_opt = bundle_to_consider;
                }
                return true;
            }
        }
        return false;
    }
}
