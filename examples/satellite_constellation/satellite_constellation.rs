use a_sabr::bundle::Bundle;
use a_sabr::contact_manager::evl::EVLManager;
use a_sabr::distance::sabr::SABR;
use a_sabr::node_manager::none::NoManagement;
use a_sabr::node_manager::NodeManager;
use a_sabr::parsing::coerce_nm;
use a_sabr::parsing::NodeDispatcher;
use a_sabr::parsing::{DispatchParser, Dispatcher, Lexer, Parser, ParsingState};
use a_sabr::pathfinding::mpt::MptPath;
use a_sabr::pathfinding::Pathfinding;
use a_sabr::types::Date;
use a_sabr::utils::{init_pathfinding, pretty_print};
struct NoRetention {}

impl NodeManager for NoRetention {
    #[cfg(feature = "node_tx")]
    fn dry_run_tx(&self, waiting_since: Date, start: Date, end: Date, bundle: &Bundle) -> bool {
        return waiting_since == start;
    }

    #[cfg(feature = "node_tx")]
    fn schedule_tx(
        &mut self,
        waiting_since: Date,
        start: Date,
        end: Date,
        bundle: &Bundle,
    ) -> bool {
        return waiting_since == start;
    }
}

/// Implements the DispatchParser to allow dynamic parsing.
impl DispatchParser<NoRetention> for NoRetention {}

/// The parser doesn't need to read tokens.
impl Parser<NoRetention> for NoRetention {
    fn parse(_lexer: &mut dyn Lexer) -> ParsingState<NoRetention> {
        ParsingState::Finished(NoRetention {})
    }
}

fn edge_case_example<NM: NodeManager + Parser<NM> + DispatchParser<NM>>(
    cp_path: &str,
    node_marker_map: Option<&Dispatcher<'_, fn(&mut dyn Lexer) -> ParsingState<NM>>>,
) {
    let bundle = Bundle {
        source: 0,
        destinations: vec![2],
        priority: 0,
        size: 0.0,
        expiration: 1000.0,
    };

    let mut mpt_graph = init_pathfinding::<NM, EVLManager, MptPath<NM, EVLManager, SABR>>(
        &cp_path,
        node_marker_map,
        None,
    );

    println!("");
    println!(
        "Running with contact plan location={}, and destination node=2 ",
        cp_path
    );

    let res = mpt_graph.get_next(0.0, 0, &bundle, &vec![]);

    match res.by_destination[2].clone() {
        Some(route) => pretty_print(route),
        None => println!("No route found to node 2."),
    }
}

fn main() {
    #[cfg(not(feature = "node_tx"))]
    panic!("Please enable the node_tx feature.");

    let mut node_dispatch: Dispatcher<NodeDispatcher> = Dispatcher::<NodeDispatcher>::new();
    node_dispatch.add("noret", coerce_nm::<NoRetention>);
    node_dispatch.add("none", coerce_nm::<NoManagement>);

    edge_case_example::<NoManagement>("examples/satellite_constellation/contact_plan_1.cp", None);
    edge_case_example::<Box<dyn NodeManager>>(
        "examples/satellite_constellation/contact_plan_2.cp",
        Some(&node_dispatch),
    );
}
