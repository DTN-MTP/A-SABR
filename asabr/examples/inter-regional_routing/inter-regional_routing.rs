use std::{
    cell::RefCell,
    fs::File,
    io::{BufRead, BufReader},
    rc::Rc,
};

use a_sabr::{
    bundle::Bundle,
    contact_plan::asabr_file_lexer::parse_from_iter,
    multigraph::Multigraph,
    node_manager::none::NoManagement,
    parsing::CMDynStandard,
    route_storage::cache::TreeCache,
    routing::{Router, aliases::SpsnHybridParenting},
};

fn main() {
    let cp_path = "asabr/examples/inter-regional_routing/asabr_format_dynamic.cp";
    // All nodes will have the same management approach (NoManagement) but the contacts may be of various types
    // We provide a map with markers that will allow the parser to create the correct contacts types thanks to
    // the markers provides in the contact plan
    // The manager type should be Box<dyn ContactManager>> (heap allocated, dynamically dispatched)
    // Replace None with a dispatching map for the contact_marker_map argument
    let file = File::open(cp_path).unwrap();
    let lines = BufReader::new(file).lines().map(|l| l.unwrap());

    let contact_plan = parse_from_iter::<NoManagement, CMDynStandard, _>(lines).unwrap();
    println!(
        "A-SABR CP parsed (statically for nodes, dynamically for contacts), found {} nodes (no management) & {} contacts (of various types)",
        contact_plan.vertices.len(),
        contact_plan.contacts.len()
    );

    println!("Virtual nodes map:");
    dbg!(&contact_plan.vnode_map);

    println!("\n---\n");

    let graph = Multigraph::new(contact_plan).unwrap();
    println!("{graph}");

    // Re-parse for the router, which consumes the contact plan to build its own multigraph.
    let file = File::open(cp_path).unwrap();
    let lines = BufReader::new(file).lines().map(|l| l.unwrap());

    let contact_plan = parse_from_iter::<NoManagement, CMDynStandard, _>(lines).unwrap();

    println!("\n---\n");

    // We create a storage for the Paths
    let table = Rc::new(RefCell::new(TreeCache::new(true, false, 10)));
    // We initialize the routing algorithm with the storage and the contacts/nodes created thanks to the parser
    let mut spsn = SpsnHybridParenting::new(contact_plan, table, false).unwrap();

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
                println!("{}", route_rc.borrow());
            }
        }
    }
}
