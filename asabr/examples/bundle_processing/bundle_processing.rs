static_assertions::assert_cfg!(feature = "node_proc");
static_assertions::assert_cfg!(not(feature = "node_tx"));
static_assertions::assert_cfg!(not(feature = "node_rx"));

use std::fs::File;
use std::io::{BufRead, BufReader};

use a_sabr::bundle::Bundle;
use a_sabr::contact_manager::legacy::evl::EVLManager;
use a_sabr::distance::sabr::SABR;
use a_sabr::errors::ASABRError;
use a_sabr::node_manager::NodeManager;
use a_sabr::node_manager::none::NoManagement;
use a_sabr::parsing::LexFrom;
use a_sabr::pathfinding::Pathfinding;
use a_sabr::pathfinding::hybrid_parenting::HybridParentingPath;
use a_sabr::types::Date;
use a_sabr::types::Priority;
use a_sabr::utils::init_pathfinding;
use a_sabr::{choices, parse_transparent, transparent_NM};

#[derive(Debug)]
struct Compressing {
    max_priority: Priority,
}

impl NodeManager for Compressing {
    fn dry_run_process(&self, at_time: Date, bundle: &mut Bundle) -> Date {
        let mut earliest_tx_time = at_time;
        if bundle.priority <= self.max_priority {
            bundle.size *= 0.75;
            earliest_tx_time += 2.0;
        }
        earliest_tx_time
    }

    fn schedule_process(&self, at_time: Date, bundle: &mut Bundle) -> Date {
        let mut earliest_tx_time = at_time;
        if bundle.priority <= self.max_priority {
            bundle.size *= 0.75;
            earliest_tx_time += 2.0;
        }
        earliest_tx_time
    }

    // The following 4 implementations are provided just to make the rust_analyzer happy
    #[cfg(feature = "node_tx")]
    fn dry_run_tx(&self, _waiting_since: Date, _start: Date, _end: Date, _bundle: &Bundle) -> bool {
        unreachable!()
    }

    #[cfg(feature = "node_tx")]
    fn schedule_tx(
        &mut self,
        _waiting_since: Date,
        _start: Date,
        _end: Date,
        _bundle: &Bundle,
    ) -> bool {
        unreachable!();
    }

    #[cfg(feature = "node_rx")]
    fn dry_run_rx(&self, _start: Date, _end: Date, _bundle: &Bundle) -> bool {
        unreachable!();
    }
    #[cfg(feature = "node_rx")]
    fn schedule_rx(&mut self, _start: Date, _end: Date, _bundle: &Bundle) -> bool {
        unreachable!();
    }
}

impl From<Priority> for Compressing {
    fn from(value: Priority) -> Self {
        Compressing {
            max_priority: value,
        }
    }
}

parse_transparent!(Compressing, Priority);

struct CompressingOrNone(Box<dyn NodeManager>);

transparent_NM!(CompressingOrNone);

choices!(
    choice,
    Choice,
    (Void, NoManagement),
    (Compress, Compressing)
);

impl TryFrom<&str> for choice::Kinds {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "none" => Ok(Self::Void),
            "compress" => Ok(Self::Compress),
            _ => Err(()),
        }
    }
}

impl From<choice::Choice> for CompressingOrNone {
    fn from(value: choice::Choice) -> Self {
        CompressingOrNone(match value {
            choice::Choice::Void(no_management) => Box::new(no_management),
            choice::Choice::Compress(compressing) => Box::new(compressing),
        })
    }
}
parse_transparent!(CompressingOrNone, choice::Choice);

fn edge_case_example<NM: NodeManager + LexFrom<str>>(
    cp_path: &str,
    bundle_priority: Priority,
) -> Result<(), ASABRError> {
    let bundle = Bundle {
        source: 0,
        destinations: vec![3],
        priority: bundle_priority,
        size: 100.0,
        expiration: 1000.0,
    };
    let file = File::open(cp_path).unwrap();
    let lines = BufReader::new(file).lines().map(|l| {
        l.map_err(|e| eprintln!("Error while reading file: {e}"))
            .unwrap()
    });
    let mut mpt_graph =
        init_pathfinding::<NM, EVLManager, HybridParentingPath<NM, EVLManager, SABR>, _, _>(lines)?;

    println!(
        "\nRunning with contact plan location={cp_path}, destination node=3, and bundle priority={bundle_priority}"
    );

    let res = mpt_graph.get_next(0.0, 0, &bundle, &[]).unwrap();

    match res.by_destination[3].clone() {
        Some(route) => print!("{}", route.borrow()),
        _ => println!("No route found to node 3."),
    }

    Ok(())
}

fn main() -> Result<(), ASABRError> {
    edge_case_example::<NoManagement>("asabr/examples/bundle_processing/contact_plan_1.cp", 0)?;
    edge_case_example::<CompressingOrNone>(
        "asabr/examples/bundle_processing/contact_plan_2.cp",
        0,
    )?;
    edge_case_example::<CompressingOrNone>(
        "asabr/examples/bundle_processing/contact_plan_2.cp",
        2,
    )?;

    Ok(())

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
