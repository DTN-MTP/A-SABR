use a_sabr::{
    bundle::Bundle,
    contact_manager::legacy::evl::EVLManager,
    distance::sabr::SABR,
    node_manager::none::NoManagement,
    pathfinding::{
        hybrid_parenting::HybridParentingPath, node_parenting::NodeParentingPath, Pathfinding,
    },
    types::NodeID,
    utils::{init_pathfinding, pretty_print},
};

#[cfg(feature = "contact_work_area")]
use a_sabr::pathfinding::contact_parenting::ContactParentingPath;

fn edge_case_example(cp_path: &str, dest: NodeID) {
    let bundle = Bundle {
        source: 0,
        destinations: vec![dest],
        priority: 0,
        size: 0.0,
        expiration: 1000.0,
    };

    let mut node_graph = init_pathfinding::<
        NoManagement,
        EVLManager,
        NodeParentingPath<NoManagement, EVLManager, SABR>,
    >(&cp_path, None, None);

    #[cfg(feature = "contact_work_area")]
    let mut contact_graph = init_pathfinding::<
        NoManagement,
        EVLManager,
        ContactParentingPath<NoManagement, EVLManager, SABR>,
    >(&cp_path, None, None);
    let mut mpt_graph = init_pathfinding::<
        NoManagement,
        EVLManager,
        HybridParentingPath<NoManagement, EVLManager, SABR>,
    >(&cp_path, None, None);

    println!("");
    println!(
        "Running with contact plan location={}, and destination node={} ",
        cp_path, dest
    );
    println!("");
    let res = node_graph.get_next(0.0, 0, &bundle, &vec![]).unwrap();
    print!("With NodeParentingPath pathfinding. ");
    pretty_print(res.by_destination[dest as usize].clone().unwrap());

    #[cfg(feature = "contact_work_area")]
    {
        let res = contact_graph.get_next(0.0, 0, &bundle, &vec![]).unwrap();
        print!("With ContactParentingPath pathfinding. ");
        pretty_print(res.by_destination[dest as usize].clone().unwrap());
    }

    let res = mpt_graph.get_next(0.0, 0, &bundle, &vec![]).unwrap();
    print!("With HybridParentingPath pathfinding. ");
    pretty_print(res.by_destination[dest as usize].clone().unwrap());
}

fn main() {
    #[cfg(not(feature = "contact_work_area"))]
    panic!("Please enable the 'contact_work_area' feature.");

    edge_case_example("examples/dijkstra_accuracy/contact_plan_1.cp", 3);
    edge_case_example("examples/dijkstra_accuracy/contact_plan_2.cp", 4);

    println!("\nN.B.: Results with the single end-to-end \"Path\" variant. We would get the same results with their \"Tree\" versions.");

    // === OUTPUT ===
    // Running with contact plan location=examples/dijkstra_accuracy/contact_plan_1.cp, and destination node=3

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

    // Running with contact plan location=examples/dijkstra_accuracy/contact_plan_2.cp, and destination node=4

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
