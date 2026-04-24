use std::{cell::RefCell, rc::Rc};

use crate::{
    contact_manager::ContactManager,
    contact_plan::{asabr_file_lexer::FileLexer, from_asabr_lexer::ASABRContactPlan},
    errors::ASABRError,
    multigraph::Multigraph,
    node_manager::NodeManager,
    parsing::{DispatchParser, Parser, StaticMarkerMap},
    pathfinding::Pathfinding,
    route_stage::SharedRouteStage,
};

pub fn init_pathfinding<
    NM: NodeManager + DispatchParser<NM> + Parser<NM>,
    CM: ContactManager + DispatchParser<CM> + Parser<CM>,
    P: Pathfinding<NM, CM>,
>(
    cp_path: &str,
    node_marker_map: Option<&StaticMarkerMap<NM>>,
    contact_marker_map: Option<&StaticMarkerMap<CM>>,
) -> Result<P, ASABRError> {
    let mut mylexer = FileLexer::new(cp_path).unwrap();
    let nodes_n_contacts =
        ASABRContactPlan::parse::<NM, CM>(&mut mylexer, node_marker_map, contact_marker_map)
            .unwrap();

    Ok(P::new(Rc::new(RefCell::new(Multigraph::new(
        nodes_n_contacts,
    )?))))
}

pub fn pretty_print_multigraph<NM: NodeManager, CM: ContactManager>(graph: &Multigraph<NM, CM>) {
    let real_node_count = graph.real_nodes.len();
    let label = |vid: crate::vertex::VertexID| -> String {
        if (vid as usize) < real_node_count {
            let node = graph.real_nodes[vid as usize].borrow();
            format!("node {} \"{}\"", node.info.id, node.info.name)
        } else {
            format!("vnode {}", vid)
        }
    };

    println!(
        "Multigraph: {} vertices ({} real node(s), {} vnode(s))",
        graph.get_vertex_count(),
        real_node_count,
        graph.get_vertex_count() - real_node_count,
    );

    for sender in &graph.senders {
        println!("- Sender {}:", label(sender.vertex_id));
        for receiver in &sender.receivers {
            println!(
                "    -> Receiver {} ({} contact(s)):",
                label(receiver.vertex_id),
                receiver.contacts_to_receiver.len(),
            );
            for contact_rc in &receiver.contacts_to_receiver {
                let c = contact_rc.borrow();
                println!(
                    "        * tx={} rx={} [{}, {}]",
                    c.info.tx_node_id, c.info.rx_node_id, c.info.start, c.info.end,
                );
            }
        }
    }
}

pub fn pretty_print<NM: NodeManager, CM: ContactManager>(route: SharedRouteStage<NM, CM>) {
    let mut backtrace: Vec<String> = Vec::new();
    println!(
        "Route to node {} at t={} with {} hop(s): ",
        route.borrow().to_node,
        route.borrow().at_time,
        route.borrow().hop_count
    );
    let mut curr_route_opt = Some(route);
    while let Some(curr_route_rc) = curr_route_opt.take() {
        let curr_route = curr_route_rc.borrow();
        backtrace.push(format!(
            "\t- Reach node {} at t={} with {} hop(s)",
            curr_route.to_node, curr_route.at_time, curr_route.hop_count
        ));
        match &curr_route.via {
            Some(via_val) => curr_route_opt = Some(via_val.parent_route.clone()),
            None => curr_route_opt = None,
        }
    }
    println!(
        "{}",
        backtrace.into_iter().rev().collect::<Vec<_>>().join("\n")
    );
}
