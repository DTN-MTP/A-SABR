use a_sabr::bundle::Bundle;
use a_sabr::contact_manager::legacy::evl::EVLManager;
use a_sabr::distance::sabr::SABR;
use a_sabr::node_manager::NodeManager;
use a_sabr::node_manager::none::NoManagement;
use a_sabr::node_manager::{NodeRx, NodeTx};
use a_sabr::parsing::NodeMarkerMap;
use a_sabr::parsing::coerce_nm;
use a_sabr::parsing::{DispatchParser, Lexer, Parser, ParsingState, StaticMarkerMap};
use a_sabr::pathfinding::Pathfinding;
use a_sabr::pathfinding::hybrid_parenting::HybridParentingPath;
use a_sabr::types::Date;
use a_sabr::types::Duration;
use a_sabr::types::Token;
use a_sabr::utils::{init_pathfinding, pretty_print};
use a_sabr_macros::{DefaultNodeManager, DefaultNodeRx};

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(DefaultNodeRx, DefaultNodeManager)]
struct NoRetention {
    max_proc_time: Duration,
}

impl NodeTx for NoRetention {
    fn dry_run_tx(&self, waiting_since: Date, start: Date, _end: Date, _bundle: &Bundle) -> bool {
        start - waiting_since < self.max_proc_time
    }

    fn schedule_tx(
        &mut self,
        waiting_since: Date,
        start: Date,
        _end: Date,
        _bundle: &Bundle,
    ) -> bool {
        start - waiting_since < self.max_proc_time
    }
}

/// Implements the DispatchParser to allow dynamic parsing.
impl DispatchParser<NoRetention> for NoRetention {}

impl Parser<NoRetention> for NoRetention {
    fn parse(lexer: &mut dyn Lexer) -> ParsingState<NoRetention> {
        // read the next token as a Duration (alias for f64)
        let max = <Duration as Token<Duration>>::parse(lexer);
        // treat success/error cases
        match max {
            ParsingState::Finished(value) => ParsingState::Finished(NoRetention {
                max_proc_time: value,
            }),
            ParsingState::Error(msg) => ParsingState::Error(msg),
            ParsingState::EOF => {
                ParsingState::Error(format!("Parsing failed ({})", lexer.get_current_position()))
            }
        }
    }
}

fn edge_case_example<NM: NodeManager + Parser<NM> + DispatchParser<NM>>(
    cp_path: &str,
    node_marker_map: Option<&StaticMarkerMap<NM>>,
) {
    let bundle = Bundle {
        source: 0,
        destinations: vec![2],
        priority: 0,
        size: 0.0,
        expiration: 1000.0,
    };

    let mut mpt_graph = init_pathfinding::<NM, EVLManager, HybridParentingPath<NM, EVLManager, SABR>>(
        cp_path,
        node_marker_map,
        None,
    );

    println!(
        "\nRunning with contact plan location={}, and destination node=2 ",
        cp_path
    );

    let res = mpt_graph.get_next(0.0, 0, &bundle, &[]).unwrap();

    match res.by_destination[2].clone() {
        Some(route) => pretty_print(route),
        _ => println!("No route found to node 2."),
    }
}

fn main() {
    let mut node_dispatch: NodeMarkerMap = NodeMarkerMap::new();
    node_dispatch.add("noret", coerce_nm::<NoRetention>);
    node_dispatch.add("none", coerce_nm::<NoManagement>);

    edge_case_example::<NoManagement>("examples/satellite_constellation/contact_plan_1.cp", None);
    edge_case_example::<Box<dyn NodeManager>>(
        "examples/satellite_constellation/contact_plan_2.cp",
        Some(&node_dispatch),
    );

    // === OUTPUT ===
    // Running with contact plan location=examples/satellite_constellation/contact_plan_1.cp, and destination node=2
    // Route to node 2 at t=11 with 2 hop(s):
    //         - Reach node 0 at t=0 with 0 hop(s)
    //         - Reach node 1 at t=0 with 1 hop(s)
    //         - Reach node 2 at t=11 with 2 hop(s)

    // Running with contact plan location=examples/satellite_constellation/contact_plan_2.cp, and destination node=2
    // No route found to node 2.
}
