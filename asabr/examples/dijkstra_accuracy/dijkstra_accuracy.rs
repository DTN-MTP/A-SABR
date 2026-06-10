assert_cfg!(feature = "contact_work_area");

use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use a_sabr::{
    bundle::Bundle,
    contact_manager::legacy::evl::EVLManager,
    distance::sabr::SABR,
    errors::ASABRError,
    node_manager::none::NoManagement,
    pathfinding::{
        Pathfinding, hybrid_parenting::HybridParentingPath, node_parenting::NodeParentingPath,
    },
    types::NodeID,
    utils::init_pathfinding,
};

use a_sabr::pathfinding::contact_parenting::ContactParentingPath;
use static_assertions::assert_cfg;

fn edge_case_example(cp_path: &str, dest: NodeID) -> Result<(), ASABRError> {
    let bundle = Bundle {
        source: 0,
        destinations: vec![dest],
        priority: 0,
        size: 0.0,
        expiration: 1000.0,
    };
    let file = File::open(cp_path).unwrap();
    let lines = BufReader::new(file).lines().map(|l| l.unwrap());

    let mut node_graph = init_pathfinding::<
        NoManagement,
        EVLManager,
        NodeParentingPath<NoManagement, EVLManager, SABR>,
        _,
        _,
    >(lines)?;

    let mut contact_graph = {
        let file = File::open(cp_path).unwrap();
        let lines = BufReader::new(file).lines().map(|l| l.unwrap());

        init_pathfinding::<
            NoManagement,
            EVLManager,
            ContactParentingPath<NoManagement, EVLManager, SABR>,
            _,
            _,
        >(lines)?
    };

    let file = File::open(cp_path).unwrap();
    let lines = BufReader::new(file).lines().map(|l| l.unwrap());

    let mut mpt_graph = init_pathfinding::<
        NoManagement,
        EVLManager,
        HybridParentingPath<NoManagement, EVLManager, SABR>,
        _,
        _,
    >(lines)?;

    println!("\nRunning with contact plan location={cp_path}, and destination node={dest} ");
    let res = node_graph.get_next(0.0, 0, &bundle, &[]).unwrap();
    print!("\nWith NodeParentingPath pathfinding. ");
    println!(
        "{}",
        res.by_destination[dest as usize].clone().unwrap().borrow()
    );

    {
        let res = contact_graph.get_next(0.0, 0, &bundle, &[]).unwrap();
        print!("With ContactParentingPath pathfinding. ");
        println!(
            "{}",
            res.by_destination[dest as usize].clone().unwrap().borrow()
        );
    }

    let res = mpt_graph.get_next(0.0, 0, &bundle, &[]).unwrap();
    print!("With HybridParentingPath pathfinding. ");
    println!(
        "{}",
        res.by_destination[dest as usize].clone().unwrap().borrow()
    );

    Ok(())
}

fn main() -> Result<(), ASABRError> {
    edge_case_example("asabr/examples/dijkstra_accuracy/contact_plan_1.cp", 3)?;
    edge_case_example("asabr/examples/dijkstra_accuracy/contact_plan_2.cp", 4)?;

    println!(
        "\nN.B.: Results with the single end-to-end \"Path\" variant. We would get the same results with their \"Tree\" versions."
    );

    Ok(())

    // === OUTPUT ===
    // Running with contact plan location=asabr/examples/dijkstra_accuracy/contact_plan_1.cp, and destination node=3

    // With NodeParentingPath pathfinding. Route to node 3 at t=30 with 3 hop(s):
    //         - Reach node 0 at t=0 with 0 hop(s)
    //         - Reach node 1 at t=0 with 1 hop(s)
    //         - Reach node 2 at t=10 with 2 hop(s)
    //         - Reach node 3 at t=30 with 3 hop(s)
    // With ContactParentingPath pathfinding. Route to node 3 at t=30 with 2 hop(s):
    //         - Reach node 0 at t=0 with 0 hop(s)
    //         - Reach node 2 at t=25 with 1 hop(s)
    //         - Reach node 3 at t=30 with 2 hop(s)
    // With HybridParentingPath pathfinding. Route to node 3 at t=30 with 2 hop(s):
    //         - Reach node 0 at t=0 with 0 hop(s)
    //         - Reach node 2 at t=25 with 1 hop(s)
    //         - Reach node 3 at t=30 with 2 hop(s)

    // Running with contact plan location=asabr/examples/dijkstra_accuracy/contact_plan_2.cp, and destination node=4

    // With NodeParentingPath pathfinding. Route to node 4 at t=50 with 4 hop(s):
    //         - Reach node 0 at t=0 with 0 hop(s)
    //         - Reach node 1 at t=0 with 1 hop(s)
    //         - Reach node 2 at t=10 with 2 hop(s)
    //         - Reach node 3 at t=20 with 3 hop(s)
    //         - Reach node 4 at t=50 with 4 hop(s)
    // With ContactParentingPath pathfinding. Route to node 4 at t=50 with 4 hop(s):
    //         - Reach node 0 at t=0 with 0 hop(s)
    //         - Reach node 1 at t=0 with 1 hop(s)
    //         - Reach node 2 at t=10 with 2 hop(s)
    //         - Reach node 3 at t=20 with 3 hop(s)
    //         - Reach node 4 at t=50 with 4 hop(s)
    // With HybridParentingPath pathfinding. Route to node 4 at t=50 with 3 hop(s):
    //         - Reach node 0 at t=0 with 0 hop(s)
    //         - Reach node 2 at t=25 with 1 hop(s)
    //         - Reach node 3 at t=25 with 2 hop(s)
    //         - Reach node 4 at t=50 with 3 hop(s)

    // N.B.: Results with the single end-to-end "Path" variant. We would get the same results with their "Tree" versions.
}
