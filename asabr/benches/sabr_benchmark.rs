use std::{
    fs::File,
    io::{BufRead, BufReader},
    time::{Duration, Instant},
};

use a_sabr::{
    bundle::Bundle,
    contact_manager::segmentation::seg::SegmentationManager,
    contact_plan::asabr_file_lexer::parse_from_iter,
    distance::sabr::SABR,
    errors::ASABRError,
    mk_router_from_cp,
    multigraph::NodeRef,
    node_manager::{delay_heuristic::DelayHeuristicManager, none::NoManagement},
};
use generativity::make_guard;

use criterion::{Criterion, black_box, criterion_group, criterion_main};

pub fn benchmark(c: &mut Criterion) {
    let file = File::open("benches/astar_graphs/100.cp").unwrap();
    let lines = BufReader::new(file).lines().map(|l| l.unwrap());
    let contact_plan =
        parse_from_iter::<DelayHeuristicManager<NoManagement>, SegmentationManager>(lines).unwrap();

    let source = 0.into();
    let destination = 60.into();
    let bundle = Bundle {
        priority: 0,
        size: 1_000,
        expiration: 24060,
    };
    let curr_time = 60;

    let mut router_types = vec![
        ("SpsnNodeParenting", Some(10)),
        ("SpsnHybridParenting", Some(10)),
        ("SpsnContactParenting", Some(10)),
    ];

    #[cfg(feature = "contact_suppression")]
    router_types.extend([
        ("CgrFirstEndingNodeParenting", None),
        ("CgrFirstEndingHybridParenting", None),
        ("CgrFirstEndingContactParenting", None),
    ]);

    #[cfg(feature = "first_depleted")]
    router_types.extend([
        ("CgrFirstDepletedNodeParenting", None),
        ("CgrFirstDepletedHybridParenting", None),
        ("CgrFirstDepletedContactParenting", None),
    ]);

    router_types.extend([
        ("VolCgrNodeParenting", None),
        ("VolCgrHybridParenting", None),
        ("VolCgrContactParenting", None),
    ]);

    // Builds a router and routes 100 times. Only the routing is timed.
    // Being a closure returning a Result, it lets us use `?` on the macro.
    let run_once = |router_name: &str, param: Option<usize>| -> Result<Duration, ASABRError> {
        make_guard!(id);

        let mut router = mk_router_from_cp!(
            id,
            DelayHeuristicManager<NoManagement>,
            SegmentationManager,
            1,
            router_name,
            contact_plan.clone(),
            param,
            SABR
        )?;

        let Ok(NodeRef::I(src)) = router.node_id_ref(source) else {
            return Err(ASABRError::ContactPlanError("No source node"));
        };
        router.set_source(src)?;

        let Ok(dest) = router.node_id_ref(destination) else {
            return Err(ASABRError::ContactPlanError("No destination node"));
        };
        let dest = dest.routable()?;

        // only the routing is timed
        let start = Instant::now();
        for _ in 0..100 {
            black_box(router.route(
                black_box(dest),
                black_box(curr_time),
                black_box(&bundle),
                black_box(None),
            )?);
        }
        Ok(start.elapsed())
    };

    let mut group = c.benchmark_group("Routers");

    for (name, param) in router_types {
        group.bench_function(name, |b| {
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    total += run_once(name, param).unwrap_or_else(|e| panic!("{}", e));
                }
                total
            });
        });
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(50);
    targets = benchmark
}
criterion_main!(benches);
