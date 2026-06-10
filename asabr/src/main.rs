//setup the allocator for the no-std lib
use std::alloc::System;

#[global_allocator]
static GLOBAL: System = System;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process::exit;
use std::{cell::RefCell, env, rc::Rc};

use a_sabr::contact_plan::{ContactPlan, asabr_file_lexer};
use a_sabr::parsing::CMDynStandard;
use a_sabr::{
    bundle::Bundle,
    errors::ASABRError,
    node_manager::none::NoManagement,
    route_storage::cache::TreeCache,
    routing::{Router, aliases::SpsnHybridParenting},
};

fn main() -> Result<(), ASABRError> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <cp_file>", args[0]);
        std::process::exit(1);
    }
    println!("Working with cp {}.", args[1]);

    let file = File::open(&args[1]).unwrap();

    // We parse the contact plan (A-SABR format)
    let contact_plan: ContactPlan<NoManagement, CMDynStandard> =
        asabr_file_lexer::parse_from_iter(BufReader::new(file).lines().map(|r| {
            r.map_err(|e| {
                eprintln!("Error while reading file: {e}");
                exit(-1)
            })
            .unwrap()
        }))?;

    // We create a storage for the Paths
    let table = Rc::new(RefCell::new(TreeCache::new(true, false, 10)));
    // We initialize the routing algorithm with the storage and the contacts/nodes created thanks to the parser
    let mut spsn = SpsnHybridParenting::new(contact_plan, table, false)?;

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
                println!("{}", route_rc.borrow());
            }
        }
    }

    Ok(())
}
