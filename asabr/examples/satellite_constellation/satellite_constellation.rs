use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;

use a_sabr::bundle::Bundle;
use a_sabr::choices;
use a_sabr::contact_manager::legacy::evl::EVLManager;
use a_sabr::distance::sabr::SABR;
use a_sabr::errors::ASABRError;
use a_sabr::node_manager::NodeManager;
use a_sabr::node_manager::none::NoManagement;
use a_sabr::parse_transparent;
use a_sabr::parsing::LexFrom;
use a_sabr::pathfinding::Pathfinding;
use a_sabr::pathfinding::hybrid_parenting::HybridParentingPath;
use a_sabr::transparent_NM;
use a_sabr::types::Date;
use a_sabr::types::Duration;
use a_sabr::utils::init_pathfinding;
use static_assertions::assert_cfg;

#[derive(Debug)]
struct NoRetention {
    max_proc_time: Duration,
}

impl NodeManager for NoRetention {
    fn accept(
        &self,
        bundle: &Bundle,
        time: a_sabr::types::TimeInterval,
        sender: a_sabr::types::NodeID,
    ) -> bool {
        true
    }

    fn dry_run_retention(
        &self,
        bundle: &Bundle,
        reception: a_sabr::types::TimeInterval,
        sender: a_sabr::types::NodeID,
        transmition: a_sabr::types::TimeInterval,
        next: a_sabr::types::NodeID,
    ) -> bool {
        transmition.end - reception.start < self.max_proc_time
    }

    fn dry_run_multi(
        &self,
        bundle: &Bundle,
        reception: a_sabr::types::TimeInterval,
        sender: a_sabr::types::NodeID,
        transmitions: &[(a_sabr::types::TimeInterval, a_sabr::types::NodeID)],
    ) -> Option<usize> {
        let r = transmitions
            .iter()
            .enumerate()
            .take_while(|(_, trans)| trans.0.end - reception.start < self.max_proc_time)
            .last();
        Some(r.map_or(0, |(index, _)| index))
    }

    fn commit(
        &mut self,
        bundle: &Bundle,
        reception: a_sabr::types::TimeInterval,
        sender: a_sabr::types::NodeID,
        transmitions: &[(a_sabr::types::TimeInterval, a_sabr::types::NodeID)],
    ) -> Result<(), ASABRError> {
        Ok(())
    }
}

impl From<Duration> for NoRetention {
    fn from(value: Duration) -> Self {
        NoRetention {
            max_proc_time: value,
        }
    }
}

parse_transparent!(NoRetention, Duration);

struct NoRetOrNone(Box<dyn NodeManager>);

transparent_NM!(NoRetOrNone);

choices!(choice, Choice, (Void, NoManagement), (NoRet, NoRetention));

impl TryFrom<&str> for choice::Kinds {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "none" => Ok(Self::Void),
            "noret" => Ok(Self::NoRet),
            _ => Err(()),
        }
    }
}

impl From<choice::Choice> for NoRetOrNone {
    fn from(value: choice::Choice) -> Self {
        NoRetOrNone(match value {
            choice::Choice::Void(no_management) => Box::new(no_management),
            choice::Choice::NoRet(noret) => Box::new(noret),
        })
    }
}
parse_transparent!(NoRetOrNone, choice::Choice);
/// Implements the DispatchParser to allow dynamic parsing.
fn edge_case_example<NM: NodeManager + LexFrom<str>>(cp_path: &str) -> Result<(), ASABRError> {
    let bundle = Bundle {
        source: 0,
        destinations: vec![2],
        priority: 0,
        size: 0.0,
        expiration: 1000,
    };
    let file = File::open(cp_path).unwrap();
    let lines = BufReader::new(file).lines().map(|l| l.unwrap());

    let mut mpt_graph =
        init_pathfinding::<NM, EVLManager, HybridParentingPath<NM, EVLManager, SABR>, _, _>(lines)?;

    println!("\nRunning with contact plan location={cp_path}, and destination node=2 ");

    let res = mpt_graph.get_next(0.0, 0, &bundle, &[]).unwrap();

    match res.by_destination[2].clone() {
        Some(route) => println!("{}", route.borrow()),
        _ => println!("No route found to node 2."),
    }

    Ok(())
}

fn main() -> Result<(), ASABRError> {
    edge_case_example::<NoManagement>("asabr/examples/satellite_constellation/contact_plan_1.cp")?;
    edge_case_example::<NoRetOrNone>("asabr/examples/satellite_constellation/contact_plan_2.cp")?;

    Ok(())

    // === OUTPUT ===
    // Running with contact plan location=asabr/examples/satellite_constellation/contact_plan_1.cp, and destination node=2
    // Route to node 2 at t=11 with 2 hop(s):
    //         - Reach node 0 at t=0 with 0 hop(s)
    //         - Reach node 1 at t=0 with 1 hop(s)
    //         - Reach node 2 at t=11 with 2 hop(s)

    // Running with contact plan location=asabr/examples/satellite_constellation/contact_plan_2.cp, and destination node=2
    // No route found to node 2.
}
