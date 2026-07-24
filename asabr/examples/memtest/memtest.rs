use a_sabr::contact::Contact;
use a_sabr::contact_manager::legacy::evl::EVLManager;
use a_sabr::contact_manager::segmentation::seg::SegmentationManager;
use std::alloc::System;
use std::hint::black_box;

use trackalloc::PeakAlloc;

use std::fs::File;

use a_sabr::contact_plan::{ContactPlan, from_tvgutil_file};
use a_sabr::multigraph::{Multigraph, NodeRef};
use a_sabr::pathfinding::top_level::spsn::AlwaysAll;
use a_sabr::pathfinding::{HybridParenting, Pathfinding};
use a_sabr::route_storage::Cached;
use a_sabr::{
    bundle::Bundle, errors::ASABRError, node_manager::none::NoManagement,
    pathfinding::top_level::aliases::SpsnHybridParenting, route_storage::cache::TreeCache,
};
use generativity::make_guard;
#[global_allocator]
static PEAK_ALLOC: PeakAlloc<System> = PeakAlloc::system();

fn main() -> Result<(), ASABRError> {
    println!(
        "At init: Current: {}, Peak: {}",
        PEAK_ALLOC.current_usage_as_kb(),
        PEAK_ALLOC.peak_usage_as_kb()
    );
    (|| {
        let filename = "benches/ptvg_files/sample1.json";

        let file = File::open(filename).unwrap();
        let json = serde_json::from_reader(file).unwrap();

        // We parse the contact plan (A-SABR format)
        let contact_plan: ContactPlan<NoManagement, EVLManager> =
            from_tvgutil_file::TVGUtilContactPlan::parse::<NoManagement, _>(json)?;

        println!(
            "Tvg-util CP parsed, found {} nodes (no management) & {} contacts (segmentation)",
            contact_plan.vnodes.len() + contact_plan.realnodes.len(),
            contact_plan.contacts.len()
        );
        println!(
            "At contact plan: Current: {}, Peak: {}",
            PEAK_ALLOC.current_usage_as_kb(),
            PEAK_ALLOC.peak_usage_as_kb()
        );
        PEAK_ALLOC.reset_peak_usage();

        make_guard!(id_guard);
        let mut multigraph = Multigraph::new(id_guard, contact_plan).unwrap();
        println!(
            "At Multigraph: Current: {}, Peak: {}. sizeof(Manager):{}, ~size per manager:{}",
            PEAK_ALLOC.current_usage_as_kb(),
            PEAK_ALLOC.peak_usage_as_kb(),
            size_of::<Contact<EVLManager>>(),
            PEAK_ALLOC.current_usage() / 20952,
        );
        PEAK_ALLOC.reset_peak_usage();

        // We create a storage for the Paths
        let table = TreeCache::new(&multigraph, 10);
        // We initialize the routing algorithm with the storage and the contacts/nodes created thanks to the parser
        let mut spsn = SpsnHybridParenting::<3, _, _, _>::new(Cached::new(
            table,
            AlwaysAll::new(HybridParenting::new()),
        ));

        // We will route a bundle
        let b = Bundle {
            priority: 0,
            size: 4_900_000,
            expiration: 24060,
        };

        let Ok(NodeRef::I(source)) = multigraph.node_id_ref(0.into()) else {
            panic!()
        };
        let destination = multigraph.node_id_ref(79.into()).unwrap();
        let mut destination = destination.routable().unwrap();

        // We schedule the bundle (resource updates were conducted)
        let out = spsn
            .find_path(&mut multigraph, 0, source, &b, &mut destination, None)
            .unwrap();

        let _ignore = black_box(out);
        println!(
            "After find: Current: {}, Peak: {}",
            PEAK_ALLOC.current_usage_as_kb(),
            PEAK_ALLOC.peak_usage_as_kb()
        );
        Ok(())
    })()?;

    println!(
        "At exit: Current: {}, Peak: {}",
        PEAK_ALLOC.current_usage_as_kb(),
        PEAK_ALLOC.peak_usage_as_kb()
    );
    Ok(())
}
