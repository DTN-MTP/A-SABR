//setup the allocator for the no-std lib
use std::alloc::System;

#[global_allocator]
static GLOBAL: System = System;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process::exit;
use std::env;

use a_sabr::contact_plan::{ContactPlan, asabr_file_lexer};
use a_sabr::multigraph::{Multigraph, NodeRef};
use a_sabr::parsing::CMDynStandard;
use a_sabr::pathfinding::{HybridParenting, Pathfinding};
use a_sabr::route_storage::Cached;
use a_sabr::{
    bundle::Bundle,
    errors::ASABRError,
    node_manager::none::NoManagement,
    route_storage::cache::TreeCache,
    routing::{aliases::SpsnHybridParenting},
};
use generativity::make_guard;

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
    make_guard!(id_guard);
    let mut multigraph = Multigraph::new(id_guard, contact_plan)?;

    // We create a storage for the Paths
    let table = TreeCache::new(&multigraph);
    // We initialize the routing algorithm with the storage and the contacts/nodes created thanks to the parser
    let mut spsn = SpsnHybridParenting::<3,_,_,_>::new(Cached::new(table, HybridParenting::new()));

    // We will route a bundle
    let b = Bundle {
        source: 0.into(),
        priority: 0,
        size: 1,
        expiration: 10000,
    };

    let Ok(NodeRef::R(source)) = multigraph.node_id_ref(0.into()) else {
        panic!()
    };
    let Ok(mut destination) = multigraph.node_id_ref(4.into()) else {
        return Err(ASABRError::ContactPlanError("No node number 4"))
    };

    // We schedule the bundle (resource updates were conducted)
    let out = spsn.find_path(&mut multigraph,0,source,&b,&mut destination,None)?;

    if let Some(out) = out {
        println!("{:?}",out)
        // for (_contact, dest_routes) in out.first_hops.values() {
        //     for route_rc in dest_routes {
        //         println!("{}", route_rc.borrow());
        //     }
        // }
    }

    Ok(())
}
