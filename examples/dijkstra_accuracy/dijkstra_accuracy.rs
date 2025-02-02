use a_sabr::{
    bundle::Bundle,
    contact_manager::evl::EVLManager,
    distance::sabr::SABR,
    node_manager::none::NoManagement,
    pathfinding::{
        contact_graph::ContactGraphPath, mpt::MptPath, node_graph::NodeGraphPath, Pathfinding,
    },
    types::NodeID,
    utils::{init_pathfinding, pretty_print},
};

fn edge_case_exemple(cp_path: &str, dest: NodeID) {
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
        NodeGraphPath<NoManagement, EVLManager, SABR>,
    >(&cp_path);
    let mut contact_graph = init_pathfinding::<
        NoManagement,
        EVLManager,
        ContactGraphPath<NoManagement, EVLManager, SABR>,
    >(&cp_path);
    let mut mpt_graph =
        init_pathfinding::<NoManagement, EVLManager, MptPath<NoManagement, EVLManager, SABR>>(
            &cp_path,
        );

    println!("");
    println!(
        "Running with contact plan location={}, and destination node={} ",
        cp_path, dest
    );
    println!("");
    let res = node_graph.get_next(0.0, 0, &bundle, &vec![]);
    print!("With NodeGraphPath pathfinding. ");
    pretty_print(res.by_destination[dest as usize].clone().unwrap());

    let res = contact_graph.get_next(0.0, 0, &bundle, &vec![]);
    print!("With ContactGraphPath pathfinding. ");
    pretty_print(res.by_destination[dest as usize].clone().unwrap());

    let res = mpt_graph.get_next(0.0, 0, &bundle, &vec![]);
    print!("With MptPath pathfinding. ");
    pretty_print(res.by_destination[dest as usize].clone().unwrap());
}

fn main() {
    edge_case_exemple("examples/dijkstra_accuracy/contact_plan_1.cp", 3);
    edge_case_exemple("examples/dijkstra_accuracy/contact_plan_2.cp", 4);

    println!("\nN.B.: Results with the single end-to-end \"Path\" variant. We would get the same results with their \"Tree\" versions.");
}
