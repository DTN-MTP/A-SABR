use a_sabr::bundle::Bundle;
use a_sabr::contact_manager::legacy::evl::EVLManager;
use a_sabr::distance::sabr::SABR;
use a_sabr::node_manager::none::NoManagement;
use a_sabr::node_manager::NodeManager;
use a_sabr::parsing::coerce_nm;
use a_sabr::parsing::{DispatchParser, Lexer, Parser, ParsingState};
use a_sabr::parsing::{NodeMarkerMap, StaticMarkerMap};
use a_sabr::pathfinding::hybrid_parenting::HybridParentingPath;
use a_sabr::pathfinding::Pathfinding;
#[cfg(any(feature = "node_rx", feature = "node_proc", feature = "node_rx"))]
use a_sabr::types::Date;
use a_sabr::types::Priority;
use a_sabr::types::Token;
use a_sabr::utils::{init_pathfinding, pretty_print};

#[cfg_attr(feature = "debug", derive(Debug))]
struct Compressing {
    max_priority: Priority,
}

impl NodeManager for Compressing {
    #[cfg(feature = "node_proc")]
    fn dry_run_process(&self, at_time: Date, bundle: &mut Bundle) -> Date {
        let mut earliest_tx_time = at_time;
        if bundle.priority <= self.max_priority {
            bundle.size *= 0.75;
            earliest_tx_time += 2.0;
        }
        return earliest_tx_time;
    }

    #[cfg(feature = "node_proc")]
    fn schedule_process(&self, at_time: Date, bundle: &mut Bundle) -> Date {
        let mut earliest_tx_time = at_time;
        if bundle.priority <= self.max_priority {
            bundle.size *= 0.75;
            earliest_tx_time += 2.0;
        }
        return earliest_tx_time;
    }

    // The following 4 implementations are provided just to make the rust_analyzer happy
    #[cfg(feature = "node_tx")]
    fn dry_run_tx(&self, _waiting_since: Date, _start: Date, _end: Date, _bundle: &Bundle) -> bool {
        panic!("Please disable the 'node_tx' and 'node_rx' features.");
    }

    #[cfg(feature = "node_tx")]
    fn schedule_tx(
        &mut self,
        _waiting_since: Date,
        _start: Date,
        _end: Date,
        _bundle: &Bundle,
    ) -> bool {
        panic!("Please disable the 'node_tx' and 'node_rx' features.");
    }

    #[cfg(feature = "node_rx")]
    fn dry_run_rx(&self, _start: Date, _end: Date, _bundle: &Bundle) -> bool {
        panic!("Please disable the 'node_tx' and 'node_rx' features.");
    }
    #[cfg(feature = "node_rx")]
    fn schedule_rx(&mut self, _start: Date, _end: Date, _bundle: &Bundle) -> bool {
        panic!("Please disable the 'node_tx' and 'node_rx' features.");
    }
}

/// Implements the DispatchParser to allow dynamic parsing.
impl DispatchParser<Compressing> for Compressing {}

/// The parser reads a maximum priority
impl Parser<Compressing> for Compressing {
    fn parse(lexer: &mut dyn Lexer) -> ParsingState<Compressing> {
        let max_priority_state = <Priority as Token<Priority>>::parse(lexer);
        match max_priority_state {
            ParsingState::Finished(value) => {
                return ParsingState::Finished(Compressing {
                    max_priority: value,
                })
            }
            ParsingState::Error(msg) => return ParsingState::Error(msg),
            ParsingState::EOF => {
                return ParsingState::Error(format!(
                    "Parsing failed ({})",
                    lexer.get_current_position()
                ))
            }
        }
    }
}

fn edge_case_example<NM: NodeManager + Parser<NM> + DispatchParser<NM>>(
    cp_path: &str,
    bundle_priority: Priority,
    node_marker_map: Option<&StaticMarkerMap<NM>>,
) {
    let bundle = Bundle {
        source: 0,
        destinations: vec![3],
        priority: bundle_priority,
        size: 100.0,
        expiration: 1000.0,
    };

    let mut mpt_graph = init_pathfinding::<NM, EVLManager, HybridParentingPath<NM, EVLManager, SABR>>(
        &cp_path,
        node_marker_map,
        None,
    );

    println!("");
    println!(
        "Running with contact plan location={}, destination node=3, and bundle priority={} ",
        cp_path, bundle_priority
    );

    let res = mpt_graph.get_next(0.0, 0, &bundle, &vec![]).unwrap();

    match res.by_destination[3].clone() {
        Some(route) => pretty_print(route),
        _ => println!("No route found to node 3."),
    }
}

fn main() {
    #[cfg(not(feature = "node_proc"))]
    panic!("Please enable the 'node_proc' feature.");

    let mut node_dispatch: NodeMarkerMap = NodeMarkerMap::new();
    node_dispatch.add("compress", coerce_nm::<Compressing>);
    node_dispatch.add("none", coerce_nm::<NoManagement>);

    edge_case_example::<NoManagement>("examples/bundle_processing/contact_plan_1.cp", 0, None);
    edge_case_example::<Box<dyn NodeManager>>(
        "examples/bundle_processing/contact_plan_2.cp",
        0,
        Some(&node_dispatch),
    );
    edge_case_example::<Box<dyn NodeManager>>(
        "examples/bundle_processing/contact_plan_2.cp",
        2,
        Some(&node_dispatch),
    );

    // === OUTPUT ===
    // Running with contact plan location=examples/bundle_processing/contact_plan_1.cp, destination node=3, and bundle priority=0
    // No route found to node 3.

    // Running with contact plan location=examples/bundle_processing/contact_plan_2.cp, destination node=3, and bundle priority=0
    // Route to node 3 at t=252 with 3 hop(s):
    //         - Reach node 0 at t=0 with 0 hop(s)
    //         - Reach node 1 at t=100 with 1 hop(s)
    //         - Reach node 2 at t=177 with 2 hop(s)
    //         - Reach node 3 at t=252 with 3 hop(s)

    // Running with contact plan location=examples/bundle_processing/contact_plan_2.cp, destination node=3, and bundle priority=2
    // No route found to node 3.
}
