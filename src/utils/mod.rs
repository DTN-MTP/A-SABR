use std::{cell::RefCell, rc::Rc};

use crate::{
    contact_manager::ContactManager,
    contact_plan::{asabr_file_lexer::FileLexer, from_asabr_lexer::ASABRContactPlan},
    multigraph::Multigraph,
    node_manager::NodeManager,
    parsing::{DispatchParser, Dispatcher, Lexer, Parser, ParsingState},
    pathfinding::Pathfinding,
    route_stage::RouteStage,
};

pub fn init_pathfinding<
    NM: NodeManager + DispatchParser<NM> + Parser<NM>,
    CM: ContactManager + DispatchParser<CM> + Parser<CM>,
    P: Pathfinding<NM, CM>,
>(
    cp_path: &str,
    node_marker_map: Option<&Dispatcher<'_, fn(&mut dyn Lexer) -> ParsingState<NM>>>,
    contact_marker_map: Option<&Dispatcher<'_, fn(&mut dyn Lexer) -> ParsingState<CM>>>,
) -> P {
    let mut mylexer = FileLexer::new(cp_path).unwrap();
    let nodes_n_contacts =
        ASABRContactPlan::parse::<NM, CM>(&mut mylexer, node_marker_map, contact_marker_map)
            .unwrap();

    return P::new(Rc::new(RefCell::new(Multigraph::new(
        nodes_n_contacts.0,
        nodes_n_contacts.1,
    ))));
}

pub fn pretty_print<NM: NodeManager, CM: ContactManager>(route: Rc<RefCell<RouteStage<NM, CM>>>) {
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
