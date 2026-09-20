use std::alloc::System;

#[global_allocator]
static GLOBAL: System = System;

use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process::exit;

use a_sabr::contact_plan::{ContactPlan, asabr_file_lexer};
use a_sabr::mk_router;
use a_sabr::multigraph::{Multigraph, NodeRef};
use a_sabr::parsing::CMDynStandard;
use a_sabr::{bundle::Bundle, errors::ASABRError, node_manager::none::NoManagement};
use generativity::make_guard;

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

    // ---- SPSN ----
    let contact_plan_spsn = parse_cp(&args[1])?;

    make_guard!(id);
    let graph_spsn = Multigraph::new(id, contact_plan_spsn).unwrap();

    mk_router!(
        id,
        spsn_router,
        NoManagement,
        CMDynStandard,
        3,
        "SpsnHybridParenting",
        graph_spsn
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
    make_guard!(id);
    let graph_cgr = Multigraph::new(id, contact_plan_volcgr).unwrap();
    mk_router!(
        id,
        volcgr_router,
        NoManagement,
        CMDynStandard,
        3,
        "VolCgrHybridParenting",
        graph_cgr
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

    // ---- CGR FirstEnding ----
    let contact_plan_firstending = parse_cp(&args[1])?;
    make_guard!(id);
    let graph_firstending = Multigraph::new(id, contact_plan_firstending).unwrap();
    mk_router!(
        id,
        firstending_router,
        NoManagement,
        CMDynStandard,
        3,
        "CgrFirstEndingHybridParenting",
        graph_firstending
    );

    let Ok(NodeRef::I(fe_source)) = firstending_router.node_id_ref(0.into()) else {
        panic!()
    };

    let Ok(fe_dest) = firstending_router.node_id_ref(4.into()) else {
        return Err(ASABRError::ContactPlanError("No node number 4"));
    };

    let fe_dest = fe_dest.routable()?;

    let out = firstending_router.find_path(fe_dest, 0, fe_source, &b, None)?;

    println!("--- CGR FirstEnding ---");

    match out {
        Some(out) => println!("{:?}", out),
        None => println!("No route found."),
    }

    Ok(())
}
