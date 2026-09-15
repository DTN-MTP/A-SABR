use a_sabr::{
    bundle::Bundle,
    contact_manager::legacy::evl::EVLManager,
    distance::sabr::SABR,
    errors::ASABRError,
    mk_graph,
    multigraph::NodeRef,
    node_manager::none::NoManagement,
    pathfinding::{ContactParenting, HybridParenting, NodeParenting, Pathfinding},
    types::NodeID,
};

fn edge_case_example(cp_path: &str, dest: NodeID) -> Result<(), ASABRError> {
    let bundle = Bundle {
        priority: 0,
        size: 0,
        expiration: 1000,
    };
    mk_graph!(graph, NoManagement, EVLManager, cp_path, file);

    // println!("Graph: {:#?}",graph);

    let Ok(NodeRef::I(source)) = graph.node_id_ref(0.into()) else {
        panic!()
    };
    let mut dest = graph
        .node_id_ref(dest)?
        .internal()
        .ok_or(ASABRError::ContactPlanError("Not a internal Node"))?;
    let mut node_finder = NodeParenting::<SABR>::new();
    let mut contact_finder = ContactParenting::<_, _, SABR, _>::new();
    let mut mpt_finder = HybridParenting::<SABR, _, _>::new();

    println!(
        "\nRunning with contact plan location={cp_path}, and destination node={} ",
        NodeRef::from(dest)
    );
    let res = node_finder
        .find_path(&mut graph, 0, source, &bundle, &mut dest, None)?
        .ok_or(ASABRError::DryRunError(
            "No path found in node parenting test",
        ))?;
    print!("\nWith NodeParentingPath pathfinding. ");
    println!("{}", res.full_path_rev(dest.into(), &graph).unwrap());

    let res = contact_finder
        .find_path(&mut graph, 0, source, &bundle, &mut dest, None)?
        .ok_or(ASABRError::DryRunError(
            "No path found in contact parenting test",
        ))?;

    print!("With ContactParentingPath pathfinding. ");
    println!("{}", res.full_path_rev(dest.into(), &graph).unwrap());

    let res = mpt_finder
        .find_path(&mut graph, 0, source, &bundle, &mut dest, None)?
        .ok_or(ASABRError::DryRunError("No path found in hybrid test"))?;
    print!("With HybridParentingPath pathfinding. ");
    println!("{}", res.full_path_rev(dest.into(), &graph).unwrap());

    Ok(())
}

fn main() -> Result<(), ASABRError> {
    edge_case_example(
        "asabr/examples/dijkstra_accuracy/contact_plan_1.cp",
        3.into(),
    )?;
    edge_case_example(
        "asabr/examples/dijkstra_accuracy/contact_plan_2.cp",
        4.into(),
    )?;

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
