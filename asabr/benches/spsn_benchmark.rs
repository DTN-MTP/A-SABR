use std::fs::File;

use a_sabr::{
    bundle::Bundle,
    contact_manager::segmentation::seg::SegmentationManager,
    contact_plan::from_tvgutil_file::TVGUtilContactPlan,
    node_manager::none::NoManagement,
    pathfinding::{destination::RoutableDest, top_level::aliases::build_generic_router},
};
use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};

pub fn benchmark(c: &mut Criterion) {
    let ptvg_filepath = "benches/ptvg_files/sample1.json";

    let source = 0.into();
    let destinatation = 79.into();
    let bundle = Bundle {
        priority: 0,
        size: 4_900_000,
        expiration: 24060,
    };
    let curr_time = 60;

    let mut router_types = vec![
        "SpsnNodeParenting",
        "SpsnHybridParenting",
        "SpsnContactParenting",
        "SpsnNodeParentingHop",
        "SpsnHybridParentingHop",
        "SpsnContactParentingHop",
    ];

    #[cfg(feature = "contact_suppression")]
    router_types.extend([
        "CgrFirstEndingNodeParenting",
        "CgrFirstEndingHybridParenting",
        "CgrFirstEndingContactParenting",
        "CgrFirstEndingNodeParentingHop",
        "CgrFirstEndingHybridParentingHop",
        "CgrFirstEndingContactParentingHop",
    ]);

    #[cfg(feature = "first_depleted")]
    router_types.extend([
        "CgrFirstDepletedNodeParenting",
        "CgrFirstDepletedHybridParenting",
        "CgrFirstDepletedContactParenting",
        "CgrFirstDepletedNodeParentingHop",
        "CgrFirstDepletedHybridParentingHop",
        "CgrFirstDepletedContactParentingHop",
    ]);

    router_types.extend([
        "VolCgrNodeParenting",
        "VolCgrHybridParenting",
        "VolCgrContactParenting",
        "VolCgrNodeParentingHop",
        "VolCgrHybridParentingHop",
        "VolCgrContactParentingHop",
    ]);
    let file = File::open(ptvg_filepath).unwrap();
    let json = serde_json::from_reader(file).unwrap();
    let contact_plan =
        TVGUtilContactPlan::parse::<NoManagement, SegmentationManager>(json).unwrap();

    let mut group = c.benchmark_group("Routers");

    for router_type in router_types {
        group.bench_function(router_type, |b| {
            b.iter_batched_ref(
                || match unsafe {
                    build_generic_router::<3, _, _>(router_type, contact_plan.clone())
                } {
                    Ok((graph, router)) => {
                        let source = graph.node_id_ref(source).unwrap().try_into().unwrap();
                        let dest = graph
                            .node_id_ref(destinatation)
                            .unwrap()
                            .routable()
                            .unwrap();
                        (graph, router, source, dest)
                    }
                    Err(err) => panic!("{}", err),
                },
                |(graph, router, source, dest)| {
                    for _ in 0..100 {
                        black_box(dest.route(
                            black_box(graph),
                            black_box(&bundle),
                            black_box(&mut **router),
                            black_box(curr_time),
                            black_box(*source),
                            black_box(None),
                        ))
                        .unwrap();
                    }
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
