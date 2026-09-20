use std::alloc::System;

#[global_allocator]
static GLOBAL: System = System;

use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process::exit;

use a_sabr::contact_plan::{ContactPlan, asabr_file_lexer};
use a_sabr::mk_router;
use a_sabr::multigraph::{NodeRef, RoutableNodeRef};
use a_sabr::parsing::CMDynStandard;
use a_sabr::pathfinding::top_level::aliases::{SpsnHybridParenting, VolCgrHybridParenting};
use a_sabr::{bundle::Bundle, errors::ASABRError, node_manager::none::NoManagement};

fn parse_cp(path: &str) -> Result<ContactPlan<NoManagement, CMDynStandard>, ASABRError> {
    let file = File::open(path).unwrap();
    asabr_file_lexer::parse_from_iter(BufReader::new(file).lines().map(|r| {
        r.map_err(|e| {
            eprintln!("Error while reading file: {e}");
            exit(-1)
        })
        .unwrap()
    }))
}

fn main() -> Result<(), ASABRError> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <cp_file>", args[0]);
        std::process::exit(1);
    }
    println!("Working with cp {}.", args[1]);

    let b = Bundle {
        priority: 0,
        size: 1,
        expiration: 10000,
    };

    // ---- Spsn ----
    let contact_plan_spsn = parse_cp(&args[1])?;
    mk_router!(
        spsn_router,
        NoManagement,
        CMDynStandard,
        SpsnHybridParenting<3, _, _, _>,
        RoutableNodeRef,
        contact_plan_spsn,
        (10, ())
    );

    let Ok(NodeRef::I(spsn_source)) = spsn_router.node_id_ref(0.into()) else {
        panic!()
    };
    let Ok(spsn_dest) = spsn_router.node_id_ref(4.into()) else {
        return Err(ASABRError::ContactPlanError("No node number 4"));
    };
    let spsn_dest = spsn_dest.routable()?;

    let out = spsn_router.find_path(spsn_dest, 0, spsn_source, &b, None)?;
    println!("--- Spsn ---");
    match out {
        Some(out) => println!("{:?}", out),
        None => println!("No route found."),
    }

    // ---- VolCgr ----
    let contact_plan_volcgr = parse_cp(&args[1])?;
    mk_router!(
        volcgr_router,
        NoManagement,
        CMDynStandard,
        VolCgrHybridParenting<_, _, _>,
        RoutableNodeRef,
        contact_plan_volcgr,
        ((),())
    );

    let Ok(NodeRef::I(volcgr_source)) = volcgr_router.node_id_ref(0.into()) else {
        panic!()
    };
    let Ok(volcgr_dest) = volcgr_router.node_id_ref(4.into()) else {
        return Err(ASABRError::ContactPlanError("No node number 4"));
    };
    let volcgr_dest = volcgr_dest.routable()?;

    let out = volcgr_router.find_path(volcgr_dest, 0, volcgr_source, &b, None)?;
    println!("--- VolCgr ---");
    match out {
        Some(out) => println!("{:?}", out),
        None => println!("No route found."),
    }

    Ok(())
}
