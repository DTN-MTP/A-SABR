use std::{any::Any, cell::RefCell, rc::Rc};

use a_sabr::{
    bundle::Bundle,
    contact_manager::{peto::PETOManager, pevl::PEVLManager, pqd::PQDManager, ContactManager},
    contact_plan::{asabr_file_lexer::FileLexer, from_asabr_lexer::ASABRContactPlan},
    distance::hop,
    node_manager::{none::NoManagement, NodeManager},
    parsing::{coerce_cm, ContactDispatcher, Dispatcher},
    route_stage::RouteStage,
    route_storage::table::RoutingTable,
    routing::{aliases::CgrFirstEndingMpt, Router},
    types::{Duration, Volume},
    utils::pretty_print,
};

pub trait ContactManagerExt: ContactManager {
    fn get_queue_size(&self) -> [Volume; 3] {
        if let Some(mgr) = self.as_any().downcast_ref::<PEVLManager>() {
            mgr.queue_size
        } else if let Some(mgr) = self.as_any().downcast_ref::<PETOManager>() {
            mgr.queue_size
        } else if let Some(mgr) = self.as_any().downcast_ref::<PQDManager>() {
            mgr.queue_size
        } else {
            panic!("get_queue_size not implemented for this ContactManager type");
        }
    }

    fn get_mav(&self) -> [Volume; 3] {
        if let Some(mgr) = self.as_any().downcast_ref::<PEVLManager>() {
            mgr.mav
        } else if let Some(mgr) = self.as_any().downcast_ref::<PETOManager>() {
            mgr.mav
        } else if let Some(mgr) = self.as_any().downcast_ref::<PQDManager>() {
            mgr.mav
        } else {
            panic!("get_mav not implemented for this ContactManager type");
        }
    }

    fn get_delay(&self) -> Duration {
        if let Some(mgr) = self.as_any().downcast_ref::<PEVLManager>() {
            mgr.delay
        } else if let Some(mgr) = self.as_any().downcast_ref::<PETOManager>() {
            mgr.delay
        } else if let Some(mgr) = self.as_any().downcast_ref::<PQDManager>() {
            mgr.delay
        } else {
            panic!("get_delay not implemented for this ContactManager type");
        }
    }

    fn as_any(&self) -> &dyn Any;
}

impl ContactManagerExt for dyn ContactManager {
    fn get_queue_size(&self) -> [Volume; 3] {
        if let Some(mgr) = self.as_any().downcast_ref::<PEVLManager>() {
            mgr.queue_size
        } else if let Some(mgr) = self.as_any().downcast_ref::<PETOManager>() {
            mgr.queue_size
        } else if let Some(mgr) = self.as_any().downcast_ref::<PQDManager>() {
            mgr.queue_size
        } else if let Some(_mgr) = self.as_any().downcast_ref::<NoManagement>() {
            [0.0; 3]
        } else {
            panic!("get_queue_size not implemented for this ContactManager type");
        }
    }
    fn get_mav(&self) -> [Volume; 3] {
        if let Some(mgr) = self.as_any().downcast_ref::<PEVLManager>() {
            mgr.mav
        } else if let Some(mgr) = self.as_any().downcast_ref::<PETOManager>() {
            mgr.mav
        } else if let Some(mgr) = self.as_any().downcast_ref::<PQDManager>() {
            mgr.mav
        } else if let Some(_mgr) = self.as_any().downcast_ref::<NoManagement>() {
            [0.0; 3]
        } else {
            panic!("get_mav not implemented for this ContactManager type");
        }
    }
    fn get_delay(&self) -> Duration {
        if let Some(mgr) = self.as_any().downcast_ref::<PEVLManager>() {
            mgr.delay
        } else if let Some(mgr) = self.as_any().downcast_ref::<PETOManager>() {
            mgr.delay
        } else if let Some(mgr) = self.as_any().downcast_ref::<PQDManager>() {
            mgr.delay
        } else if let Some(_mgr) = self.as_any().downcast_ref::<NoManagement>() {
            0.0
        } else {
            panic!("get_delay not implemented for this ContactManager type");
        }
    }
    fn as_any(&self) -> &dyn Any {
        self // TODO
    }
}

impl ContactManagerExt for Box<dyn ContactManager> {
    fn get_queue_size(&self) -> [Volume; 3] {
        (**self).get_queue_size()
    }
    fn get_mav(&self) -> [Volume; 3] {
        (**self).get_mav()
    }
    fn get_delay(&self) -> Duration {
        (**self).get_delay()
    }
    fn as_any(&self) -> &dyn Any {
        (**self).as_any()
    }
}

/// Analyze a route and print detailed information about each hop
///
/// This function traverses the route in transmission order (from source to destination)
/// and prints detailed information about each hop, including:
/// - Data rate
/// - Queue size
/// - Maximum available volume (MAV)
/// - Transmission timing
/// - Real transmission duration
///
/// # Arguments
///
/// * `route` - The route to analyze
/// * `bundle` - The bundle being routed
///
/// # Example
///
/// ```
/// let route = cgr.route(0, &bundle, 0.0, &Vec::new()).unwrap();
/// for (_, (_, dest_routes)) in &route.first_hops {
///     for route_rc in dest_routes {
///         analyze_route(route_rc.clone(), &bundle);
///     }
/// }
/// ```
pub fn analyze_route<NM: NodeManager, CM: ContactManagerExt>(
    route: Rc<RefCell<RouteStage<NM, CM>>>,
    bundle: &Bundle,
) {
    let route_clone = route.clone();
    let route_borrowed = route.borrow();

    println!("Route Analysis:");
    println!("  Destination: Node {}", route_borrowed.to_node);
    println!("  Arrival Time: {}", route_borrowed.at_time);
    println!("  Hop Count: {}", route_borrowed.hop_count);

    let mut hops = Vec::new();
    let mut curr_route_opt = Some(route_clone);
    while let Some(curr_route_rc) = curr_route_opt.take() {
        let curr_route = curr_route_rc.borrow();
        if let Some(via) = &curr_route.via {
            hops.push(via.clone());
            curr_route_opt = Some(via.parent_route.clone());
        } else {
            curr_route_opt = None;
        }
    }
    hops.reverse();

    for (i, via) in hops.iter().enumerate() {
        let contact_borrowed = via.contact.borrow();
        let contact_info = contact_borrowed.info;
        let manager = &contact_borrowed.manager;

        println!(
            "\nHop: Node {} -> Node {}",
            contact_info.tx_node, contact_info.rx_node
        );
        println!(
            "  Contact duration: {}",
            contact_info.end - contact_info.start
        );
        // println!("  Data Rate: {:.2f}", contact_info.rate); //TODO: data rate

        println!("  Queue Size: {:?}", manager.get_queue_size());
        println!("  MAV: {:?}", manager.get_mav());
        println!("  Delay: {} time units", manager.get_delay());

        let at_time = if i == 0 {
            0.0 // TODO: For the first hop, use 0.0 as the start time for now
        } else {
            let prev_route = hops[i - 1].parent_route.borrow();
            prev_route.at_time
        };
        // TODO: avoid dry run again.
        if let Some(res) = manager.dry_run_tx(&contact_info, at_time, bundle) {
            let transmission_duration = res.tx_end - res.tx_start;
            println!("  Transmission Start: {}", res.tx_start);
            println!("  Transmission End: {}", res.tx_end);
            println!("  Transmission Duration: {}", transmission_duration);
            // println!("  Bundle Arrival: {}", res.tx_end + res.delay);

            let queue_delay = res.tx_start - at_time.max(contact_info.start);
            if queue_delay > 0.0 {
                println!("  Queue Delay: {}", queue_delay);
            } else {
                println!("  Queue Delay: no delay");
            }
        } else {
            println!("  Could not calculate transmission times");
        }
    }

    if let Some(_last_hop) = hops.last() {
        let first_hop = &hops[0];
        let start_time = first_hop.parent_route.borrow().at_time;
        let end_time = route_borrowed.at_time;
        let total_duration = end_time - start_time;

        println!("\nEnd-to-End Metrics:");
        println!("  Total Duration: {}", total_duration);
    }
}

fn edge_case_example(
    cp_path: &str,
    contact_dispatch: &Dispatcher<ContactDispatcher>,
    bundles: &[Bundle],
) {
    let mut lexer = FileLexer::new(cp_path).unwrap();
    let mut cp = ASABRContactPlan::new();
    let (nodes, contacts) = cp
        .parse::<NoManagement, Box<dyn ContactManager>>(&mut lexer, None, Some(contact_dispatch))
        .unwrap();
    let table = Rc::new(RefCell::new(RoutingTable::new()));
    let mut cgr =
        CgrFirstEndingMpt::<NoManagement, Box<dyn ContactManager>>::new(nodes, contacts, table);
    for (i, bundle) in bundles.iter().enumerate() {
        println!(
            "\nRouting bundle {} (priority: {}, size: {})",
            i + 1,
            bundle.priority,
            bundle.size
        );
        match cgr.route(0, bundle, 0.0, &Vec::new()) {
            Some(out) => {
                for (_, (_c, dest_routes)) in &out.first_hops {
                    for route_rc in dest_routes {
                        analyze_route(route_rc.clone(), bundle);
                    }
                }
            }
            None => {
                println!(
                    "No route found for bundle {} (priority: {})",
                    i + 1,
                    bundle.priority
                );
            }
        }
    }
}

fn main() {
    #[cfg(not(all(feature = "enable_priority", feature = "contact_suppression")))]
    panic!("Please enable the 'enable_priority' and the 'contact_suppression' feature.");

    let mut contact_dispatch: Dispatcher<ContactDispatcher> =
        Dispatcher::<ContactDispatcher>::new();
    contact_dispatch.add("evl", coerce_cm::<PEVLManager>);
    contact_dispatch.add("qd", coerce_cm::<PQDManager>);
    contact_dispatch.add("eto", coerce_cm::<PETOManager>);

    let bundles_a = vec![
        Bundle {
            source: 0,
            destinations: vec![3],
            priority: 2,
            size: 5.0,
            expiration: 10000.0,
        },
        Bundle {
            source: 0,
            destinations: vec![3],
            priority: 1,
            size: 10.0,
            expiration: 10000.0,
        },
        Bundle {
            source: 0,
            destinations: vec![3],
            priority: 0,
            size: 5.0,
            expiration: 10000.0,
        },
    ];

    edge_case_example(
        "examples/volume_management/contact_plan_1.cp",
        &contact_dispatch,
        &bundles_a,
    );
}