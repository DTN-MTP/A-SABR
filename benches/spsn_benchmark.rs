use a_sabr::{
    bundle::Bundle, contact_manager::segmentation::seg::SegmentationManager,
    contact_plan::from_tvgutil_file::TVGUtilContactPlan, node_manager::none::NoManagement,
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

    let mut router_types = vec![
        "SpsnHybridParenting",
        "SpsnNodeParenting",
        "SpsnHybridParentingHop",
        "SpsnNodeParentingHop",
    ];

    #[cfg(feature = "contact_work_area")]
    router_types.extend(["SpsnContactParenting", "SpsnContactParentingHop"]);

    #[cfg(feature = "contact_suppression")]
    router_types.extend([
        "CgrFirstEndingHybridParenting",
        "CgrFirstEndingNodeParentingHop",
    ]);

    #[cfg(feature = "first_depleted")]
    router_types.extend([
        "CgrFirstDepletedHybridParenting",
        "CgrFirstDepletedNodeParenting",
        "CgrFirstDepletedHybridParentingHop",
        "CgrFirstDepletedNodeParentingHop",
    ]);

    #[cfg(all(feature = "contact_work_area", feature = "contact_suppression"))]
    router_types.extend([
        "CgrFirstEndingContactParenting",
        "CgrFirstEndingContactParentingHop",
    ]);
    #[cfg(all(feature = "contact_work_area", feature = "first_depleted"))]
    router_types.extend([
        "CgrFirstDepletedContactParenting",
        "CgrFirstDepletedContactParentingHop",
    ]);

    router_types.extend([
        "VolCgrHybridParenting",
        "VolCgrNodeParenting",
        "VolCgrHybridParentingHop",
        "VolCgrNodeParentingHop",
    ]);

    #[cfg(feature = "contact_work_area")]
    router_types.extend(["VolCgrContactParenting", "VolCgrContactParentingHop"]);

    let mut group = c.benchmark_group("Routers");

    for router_type in router_types {
        group.bench_function(router_type, |b| {
            b.iter_batched(
                || {
                    let (nodes, contacts) = TVGUtilContactPlan::parse::<
                        NoManagement,
                        SegmentationManager,
                    >(ptvg_filepath)
                    .unwrap();

                    let router = match build_generic_router(
                        router_type,
                        nodes,
                        contacts,
                        Some(spsn_opts.clone()),
                    ) {
                        Ok(rter) => rter,
                        Err(err) => panic!("{}", err),
                    };
                    router
                },
                |mut router| {
                    let _ = black_box(router.route(
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
