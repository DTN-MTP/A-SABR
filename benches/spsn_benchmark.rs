use std::{cell::RefCell, rc::Rc};

use a_sabr::{
    bundle::Bundle, contact_manager::seg::SegmentationManager,
    contact_plan::from_tvgutil_file::TVGUtilContactPlan, route_storage::cache::TreeCache,
    routing::aliases::*, types::NodeID,
};
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};

pub fn benchmark(c: &mut Criterion) {
    let ptvg_filepath = "benches/ptvg_files/sample1.json";

    let source = 178;
    let bundle = Bundle {
        source: 178,
        destinations: vec![159],
        priority: 0,
        size: 47419533.0,
        expiration: 24060.0,
    };
    let curr_time = 60.0;
    let excluded_nodes: Vec<NodeID> = vec![];
    let spsn_opts = SpsnOptions {
        check_size: false,
        check_priority: false,
        max_entries: 10,
    };

    let mut router_types = vec!["SpsnMpt", "SpsnNodeGraph", "SpsnHopMpt", "SpsnHopNodeGraph"];

    #[cfg(feature = "contact_work_area")]
    router_types.extend(["SpsnContactGraph", "SpsnHopContactGraph"]);

    #[cfg(feature = "contact_suppression")]
    router_types.extend(["CgrFirstEndingMpt", "CgrHopFirstEndingNodeGraph"]);

    #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
    router_types.extend([
        "CgrFirstDepletedMpt",
        "CgrFirstDepletedNodeGraph",
        "CgrHopFirstDepletedMpt",
        "CgrHopFirstDepletedNodeGraph",
    ]);

    #[cfg(all(feature = "contact_work_area", feature = "contact_suppression"))]
    router_types.extend([
        "CgrFirstEndingContactGraph",
        "CgrHopFirstEndingContactGraph",
    ]);
    #[cfg(all(
        feature = "contact_work_area",
        feature = "contact_suppression",
        feature = "first_depleted"
    ))]
    router_types.extend([
        "CgrFirstDepletedContactGraph",
        "CgrHopFirstDepletedContactGraph",
    ]);

    let mut group = c.benchmark_group("Routers");

    for router_type in router_types {
        group.bench_function(router_type, |b| {
            b.iter_batched(
                || {
                    let (nodes, contacts) =
                        TVGUtilContactPlan::parse::<SegmentationManager>(ptvg_filepath).unwrap();

                    build_generic_router(router_type, nodes, contacts, Some(spsn_opts.clone()))
                },
                |mut router| {
                    black_box(router.route(
                        black_box(source),
                        black_box(&bundle),
                        black_box(curr_time),
                        black_box(&excluded_nodes),
                    ));
                },
                BatchSize::SmallInput,
            );
        });
    }
}

criterion_group! {
    name=benches;
    config=Criterion::default().sample_size(50);
    targets=benchmark
}
criterion_main!(benches);
