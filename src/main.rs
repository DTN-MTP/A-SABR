use std::{cell::RefCell, env, rc::Rc};

use a_sabr::{
    bundle::Bundle,
    contact_manager::{
        legacy::{eto::ETOManager, evl::EVLManager, qd::QDManager},
        segmentation::seg::SegmentationManager,
        ContactManager,
    },
    contact_plan::{asabr_file_lexer::FileLexer, from_asabr_lexer::ASABRContactPlan},
    node_manager::none::NoManagement,
    parsing::{coerce_cm, ContactMarkerMap},
    route_storage::cache::TreeCache,
    routing::{aliases::SpsnHybridParenting, Router},
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

    // All nodes will have the same management approach (NoManagement) but the contacts may be of various types
    // We provide a map with markers that will allow the parser to create the correct contacts types thanks to
    // the markers provides in the contact plan
    let mut contact_dispatch: ContactMarkerMap = ContactMarkerMap::new();
    contact_dispatch.add("evl", coerce_cm::<EVLManager>);
    contact_dispatch.add("qd", coerce_cm::<QDManager>);
    contact_dispatch.add("eto", coerce_cm::<ETOManager>);
    contact_dispatch.add("seg", coerce_cm::<SegmentationManager>);

    // We parse the contact plan (A-SABR format thanks to ASABRContactPlan) and the lexer
    let (nodes, contacts) = ASABRContactPlan::parse::<NoManagement, Box<dyn ContactManager>>(
        &mut mylexer,
        None,
        Some(&contact_dispatch),
    )
    .unwrap();

    // We create a storage for the Paths
    let table = Rc::new(RefCell::new(TreeCache::new(true, false, 10)));
    // We initialize the routing algorithm with the storage and the contacts/nodes created thanks to the parser
    let mut spsn = SpsnHybridParenting::<NoManagement, Box<dyn ContactManager>>::new(
        nodes, contacts, table, false,
    );

    // We will route a bundle
    let b = Bundle {
        source: 0,
        destinations: vec![4],
        priority: 0,
        size: 1.0,
        expiration: 10000.0,
    };

    // We schedule the bundle (resource updates were conducted)
    let out = spsn.route(0, &b, 0.0, &Vec::new());

    if let Ok(Some(out)) = out {
        for (_contact, dest_routes) in out.first_hops.values() {
            for route_rc in dest_routes {
                pretty_print(route_rc.clone());
            }
        }
    }
}
