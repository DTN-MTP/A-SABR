use std::{cell::RefCell, rc::Rc};

use a_sabr::{
    bundle::Bundle,
    contact_manager::{
        ContactManager,
        legacy::{eto::ETOManager, evl::EVLManager, qd::QDManager},
        segmentation::seg::SegmentationManager,
    },
    contact_plan::{asabr_file_lexer::FileLexer, from_asabr_lexer::ASABRContactPlan},
    multigraph::Multigraph,
    node_manager::none::NoManagement,
    parsing::{ContactMarkerMap, coerce_cm},
    route_storage::cache::TreeCache,
    routing::{Router, aliases::SpsnHybridParenting},
    utils::{pretty_print, pretty_print_multigraph},
};

fn main() {
    let cp_path = "examples/inter-regional_routing/asabr_format_dynamic.cp";
    // All nodes will have the same management approach (NoManagement) but the contacts may be of various types
    // We provide a map with markers that will allow the parser to create the correct contacts types thanks to
    // the markers provides in the contact plan
    let mut contact_dispatch: ContactMarkerMap = ContactMarkerMap::new();
    contact_dispatch.add("eto", coerce_cm::<ETOManager>);
    contact_dispatch.add("qd", coerce_cm::<QDManager>);
    contact_dispatch.add("evl", coerce_cm::<EVLManager>);
    contact_dispatch.add("seg", coerce_cm::<SegmentationManager>);

    // The manager type should be Box<dyn ContactManager>> (heap allocated, dynamically dispatched)
    // Replace None with a dispatching map for the contact_marker_map argument
    let mut mylexer = FileLexer::new(cp_path).unwrap();
    let contact_plan = ASABRContactPlan::parse::<NoManagement, Box<dyn ContactManager>>(
        &mut mylexer,
        None,
        Some(&contact_dispatch),
    )
    .unwrap();
    println!(
        "A-SABR CP parsed (statically for nodes, dynamically for contacts), found {} nodes (no management) & {} contacts (of various types)",
        contact_plan.vertices.len(),
        contact_plan.contacts.len()
    );

    println!("Virtual nodes map:");
    dbg!(&contact_plan.vnode_map);

    println!("\n---\n");

    let graph = Multigraph::new(contact_plan).unwrap();
    pretty_print_multigraph(&graph);

    // Re-parse for the router, which consumes the contact plan to build its own multigraph.
    let mut router_lexer = FileLexer::new(cp_path).unwrap();
    let contact_plan = ASABRContactPlan::parse::<NoManagement, Box<dyn ContactManager>>(
        &mut router_lexer,
        None,
        Some(&contact_dispatch),
    )
    .unwrap();

    println!("\n---\n");

    // We create a storage for the Paths
    let table = Rc::new(RefCell::new(TreeCache::new(true, false, 10)));
    // We initialize the routing algorithm with the storage and the contacts/nodes created thanks to the parser
    let mut spsn = SpsnHybridParenting::<NoManagement, Box<dyn ContactManager>>::new(
        contact_plan,
        table,
        false,
    )
    .unwrap();

    // We will route a bundle
    let b = Bundle {
        source: 0,
        destinations: vec![8],
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
