use a_sabr::bundle::Bundle;
use a_sabr::contact_manager::legacy::evl::EVLManager;
use a_sabr::distance::{astar::AStar, sabr::SABR};
use a_sabr::errors::ASABRError;
use a_sabr::mk_graph;
use a_sabr::node_manager::{delay_heuristic::DelayHeuristicManager, none::NoManagement};
use a_sabr::pathfinding::{NodeParenting, Pathfinding};

/// Every node carries a delay heuristic on top of no resource management.
type NM = DelayHeuristicManager<NoManagement>;

fn main() -> Result<(), ASABRError> {
    let cp = "asabr/examples/astar_routing/contact_plan.cp";
    let bundle = Bundle {
        priority: 0,
        size: 0,
        expiration: 1000,
    };

    mk_graph!(graph, NM, EVLManager, cp, file);

    let mut sabr_pathfinder = NodeParenting::<SABR>::new();
    let mut astar_pathfinder = NodeParenting::<AStar<SABR>>::new();

    let source = graph
        .node_id_ref(0.into())?
        .internal()
        .ok_or(ASABRError::ContactPlanError("No Node 0"))?;

    let mut destination = graph.node_id_ref(3.into())?.routable().unwrap();

    let sabr_res =
        sabr_pathfinder.find_path(&mut graph, 0, source, &bundle, &mut destination, None)?;
    let astar_res =
        astar_pathfinder.find_path(&mut graph, 0, source, &bundle, &mut destination, None)?;

    println!("\nContact plan {}:", cp,);

    match &sabr_res {
        Some(route) if let Some(route) = route.full_path_rev(destination, &graph) => {
            println!("SABR (Dijkstra): {}", route)
        }
        _ => println!("No route found to node 3 with SABR."),
    }

    match &astar_res {
        Some(route) if let Some(route) = route.full_path_rev(destination, &graph) => {
            println!("AStar<SABR>: {}", route)
        }
        _ => println!("No route found to node 3 with A*."),
    }

    Ok(())
}
