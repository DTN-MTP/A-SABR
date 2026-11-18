use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;

use a_sabr::bundle::Bundle;
use a_sabr::contact_manager::legacy::evl::EVLManager;
use a_sabr::contact_plan::asabr_file_lexer::parse_from_iter;
use a_sabr::distance::sabr::SABR;
use a_sabr::errors::ASABRError;
use a_sabr::mk_router_from_cp;
use a_sabr::node_manager::buffer::BufferManager;
use generativity::make_guard;

fn main() -> Result<(), ASABRError> {
    // Resolved from the crate directory, so the example runs from any working directory
    let file = File::open(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/buffer_routing/contact_plan.asabr"
    ))
    .unwrap();
    let lines = BufReader::new(file).lines().map(|l| l.unwrap());

    // Nodes are parsed with BufferManager (capacity given after the name),
    // contacts with EVLManager (volume-limited only, no queueing delay).
    let contact_plan = parse_from_iter::<BufferManager, EVLManager>(lines).unwrap();

    make_guard!(id);

    let mut router = mk_router_from_cp!(
        id,
        BufferManager,
        EVLManager,
        1,
        "SpsnHybridParenting",
        contact_plan,
        Some(10),
        SABR
    )?;

    let src_a = router.node_id_ref(0.into()).unwrap().internal().unwrap();
    router.set_source(src_a)?;
    let dest_d = router.node_id_ref(3.into()).unwrap().routable().unwrap();

    // b and c can each store exactly one bundle of this size
    let bundle = Bundle {
        priority: 0,
        size: 10,
        expiration: 10000,
    };

    // Bundle 1: the path through b reaches d first, and b has room.
    // The route is committed: b's buffer is booked from reception until
    // the bundle leaves on b -> d.
    println!("Routing bundle 1 from a to d at t=0");
    let (route_1, _) = router.route(dest_d, 0, &bundle, None)?.unwrap();
    println!("{}", route_1);

    // Bundle 2: b would still be fast, but its buffer is already full while
    // bundle 1 waits for b -> d. BufferManager refuses the retention, so the
    // router takes c instead of going through b a second time.
    println!("Routing bundle 2 from a to d at t=0");
    let (route_2, _) = router.route(dest_d, 0, &bundle, None)?.unwrap();
    println!("{}", route_2);

    // === OUTPUT ===
    // Routing bundle 1 from a to d at t=0
    // Route to enode: 3 at t=60 with 2 hop(s):
    //         - Reach node enode: 3 at t=60 with 2 hop(s)
    //         - Reach node enode: 1 at t=10 with 1 hop(s)
    //         - Reach node enode: 0 at t=0 with 0 hop(s)
    //
    // Routing bundle 2 from a to d at t=0
    // Route to enode: 3 at t=80 with 2 hop(s):
    //         - Reach node enode: 3 at t=80 with 2 hop(s)
    //         - Reach node enode: 2 at t=10 with 1 hop(s)
    //         - Reach node enode: 0 at t=0 with 0 hop(s)
    Ok(())
}
