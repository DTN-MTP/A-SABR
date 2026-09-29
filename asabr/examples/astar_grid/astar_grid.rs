use std::fmt::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

use a_sabr::bundle::Bundle;
use a_sabr::contact_manager::legacy::evl::EVLManager;
use a_sabr::distance::{astar::AStar, sabr::SABR};
use a_sabr::empty_parse;
use a_sabr::errors::ASABRError;
use a_sabr::mk_graph;
use a_sabr::node_manager::{NodeManager, delay_heuristic::DelayHeuristicManager};
use a_sabr::pathfinding::{NodeParenting, Pathfinding};
use a_sabr::types::{NodeID, TimeInterval};

/// Grid size: WIDTH x HEIGHT nodes, each linked to its 4 neighbours.
const WIDTH: usize = 30;
const HEIGHT: usize = 30;
const HOP_DELAY: i64 = 1;

/// Number of hops the pathfinder tried (one `accept` call per candidate hop).
static HOPS_TRIED: AtomicUsize = AtomicUsize::new(0);

/// No resource management, but counts the hops tried towards this node.
#[derive(Debug, Default)]
struct Counting;
empty_parse!(Counting);

impl NodeManager for Counting {
    fn accept(&self, _bundle: &Bundle, _time: TimeInterval, _sender: NodeID) -> bool {
        HOPS_TRIED.fetch_add(1, Ordering::Relaxed);
        true
    }
    fn dry_run_retention(
        &self,
        _bundle: &Bundle,
        _reception: TimeInterval,
        _sender: NodeID,
        _transmission: TimeInterval,
        _next: NodeID,
    ) -> bool {
        true
    }
    fn dry_run_multi(
        &self,
        _bundle: &Bundle,
        _reception: TimeInterval,
        _sender: NodeID,
        transmissions: &[(TimeInterval, NodeID)],
    ) -> Option<usize> {
        Some(transmissions.len())
    }
    fn commit(
        &mut self,
        _bundle: &Bundle,
        _reception: TimeInterval,
        _sender: NodeID,
        _transmissions: &[(TimeInterval, NodeID)],
    ) -> Result<(), ASABRError> {
        Ok(())
    }
}

type NM = DelayHeuristicManager<Counting>;

fn id(x: usize, y: usize) -> usize {
    y * WIDTH + x
}

/// A-SABR contact plan of the grid. Each node's heuristic row is its Manhattan
/// distance to every other node times the hop delay: a lower bound on the delay.
fn grid_plan() -> String {
    let mut plan = String::new();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let row: Vec<String> = (0..HEIGHT)
                .flat_map(|ty| (0..WIDTH).map(move |tx| (tx, ty)))
                .map(|(tx, ty)| ((x.abs_diff(tx) + y.abs_diff(ty)) as i64 * HOP_DELAY).to_string())
                .collect();
            writeln!(plan, "node {} n{x}_{y} [{}]", id(x, y), row.join(", ")).unwrap();
        }
    }
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let mut link = |nx: usize, ny: usize| {
                writeln!(
                    plan,
                    "contact {} {} 0 100000 1 {HOP_DELAY}",
                    id(x, y),
                    id(nx, ny)
                )
                .unwrap();
            };
            if x + 1 < WIDTH {
                link(x + 1, y);
            }
            if x > 0 {
                link(x - 1, y);
            }
            if y + 1 < HEIGHT {
                link(x, y + 1);
            }
            if y > 0 {
                link(x, y - 1);
            }
        }
    }
    plan
}

macro_rules! route {
    ($distance:ty, $plan:expr, $source:expr, $dest:expr) => {{
        let bundle = Bundle {
            priority: 0,
            size: 0,
            expiration: 1_000_000,
        };
        mk_graph!(graph, NM, EVLManager, $plan, raw);
        let source = graph.node_id_ref($source.into())?.internal().unwrap();
        let mut destination = graph.node_id_ref($dest.into())?.routable().unwrap();
        let mut finder = NodeParenting::<$distance>::new();

        HOPS_TRIED.store(0, Ordering::Relaxed);
        let res = finder.find_path(&mut graph, 0, source, &bundle, &mut destination, None)?;
        let hops_tried = HOPS_TRIED.load(Ordering::Relaxed);
        let arrival = res
            .as_ref()
            .and_then(|r| r.full_path_rev(destination, &graph).map(|p| format!("{p}")))
            .and_then(|p| p.lines().next().map(str::to_owned))
            .unwrap_or_else(|| "no route".into());

        println!(
            "{:<12} {:>11}    {}",
            stringify!($distance),
            hops_tried,
            arrival
        );
    }};
}

fn main() -> Result<(), ASABRError> {
    let plan = grid_plan();
    let source = id(0, HEIGHT / 2);
    let dest = id(WIDTH - 1, HEIGHT / 2);
    println!("{WIDTH}x{HEIGHT} grid, from node {source} to node {dest}\n");
    println!("{:<12} {:>11}    result", "distance", "hops tried",);
    route!(SABR, plan, source, dest);
    route!(AStar<SABR>, plan, source, dest);
    Ok(())
}
