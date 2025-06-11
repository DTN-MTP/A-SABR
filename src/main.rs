use std::{cell::RefCell, env, rc::Rc};

use a_sabr::{
    bundle::Bundle,
    contact_manager::{
        eto::ETOManager, evl::EVLManager, qd::QDManager, seg::SegmentationManager, ContactManager,
    },
    contact_plan::{asabr_file_lexer::FileLexer, from_asabr_lexer::ASABRContactPlan},
    node_manager::none::NoManagement,
    parsing::{coerce_cm, ContactDispatcher, Dispatcher},
    route_storage::cache::TreeCache,
    routing::{
        aliases::{CgrFirstEndingMpt, SpsnMpt},
        Router,
    },
    utils::pretty_print,
};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <cp_file>", args[0]);
        std::process::exit(1);
    }
    println!("Working with cp {}.", args[1]);

    // We create a lexer to retrieve tokens from a file
    let mut mylexer = FileLexer::new(&args[1]).unwrap();
    let mut cp = ASABRContactPlan::new();

    // All nodes will have the same management approach (NoManagement) but the contacts may be of various types
    // We provide a map with markers that will allow the parser to create the correct contacts types thanks to
    // the markers provides in the contact plan
    let mut contact_dispatch: Dispatcher<ContactDispatcher> =
        Dispatcher::<ContactDispatcher>::new();
    contact_dispatch.add("evl", coerce_cm::<EVLManager>);
    contact_dispatch.add("qd", coerce_cm::<QDManager>);
    contact_dispatch.add("evl", coerce_cm::<ETOManager>);
    contact_dispatch.add("seg", coerce_cm::<SegmentationManager>);

    // We parse the contact plan (A-SABR format thanks to ASABRContactPlan) and the lexer
    let (nodes, contacts) = cp
        .parse::<NoManagement, Box<dyn ContactManager>>(&mut mylexer, None, Some(&contact_dispatch))
        .unwrap();

    // We create a storage for the Paths
    let table = Rc::new(RefCell::new(TreeCache::new(true, false, 10)));
    // We initialize the routing algorithm with the storage and the contacts/nodes created thanks to the parser
    let mut spsn =
        SpsnMpt::<NoManagement, Box<dyn ContactManager>>::new(nodes, contacts, table, false);

    // We will route a bundle
    let b = Bundle {
        source: 0,
        destinations: vec![4],
        priority: 0, // use 0-2, 0 as lowest priority
        size: 1.0,
        expiration: 10000.0,
    };

    // We schedule the bundle (resource updates were conducted)
    let out = spsn.route(0, &b, 0.0, &Vec::new());

    if let Some(out) = out {
        for (_, (c, dest_routes)) in &out.first_hops {
            for route_rc in dest_routes {
                pretty_print(route_rc.clone());
            }
        }
    }

    let table = Rc::new(RefCell::new(RoutingTable::new())); //TODO

    let mut cgr =
        CgrFirstEndingMpt::<NoManagement, Box<dyn a_sabr::contact_manager::ContactManager>>::new(
            nodes, contacts, table,
        );

    let out = cgr.route(0, &b, 0.0, &Vec::new());
    match out {
        Some(out) => {
            // for (_, (contact_rc, dest_routes)) in &out.first_hops {
            //     for route_rc in dest_routes {
            //         pretty_print(route_rc.clone());
            //     }
            for (_, (contact_rc, dest_routes)) in &out.contacts {
                //TODO
                let contact = contact_rc.borrow();
                println!(
                    "First hop contact: from {} to {}, time [{}, {}]",
                    contact.info.rx_node,
                    contact.info.tx_node,
                    contact.info.start,
                    contact.info.end
                );
            }
        }
        None => {
            println!("  No route found for bundle (priority: {})", b.priority);
        }
    }
}
