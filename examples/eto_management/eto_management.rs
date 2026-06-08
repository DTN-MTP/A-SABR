assert_cfg!(feature = "manual_queueing");

use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;

use a_sabr::bundle::Bundle;
use a_sabr::contact_manager::ContactManager;
use a_sabr::contact_plan::asabr_file_lexer::parse_from_iter;
use a_sabr::node_manager::none::NoManagement;
use a_sabr::parsing::CMDynStandard;
use a_sabr::routing::aliases::SpsnOptions;
use a_sabr::routing::aliases::build_generic_router;
use static_assertions::assert_cfg;

fn main() {
    // We want variations for contact management, register ETO and EVL

    // We create a lexer to retrieve tokens from a file
    let file = File::open("examples/eto_management/contact_plan_1.cp").unwrap();
    let lines = BufReader::new(file).lines().map(|l| l.unwrap());

    // We parse the contact plan (A-SABR format thanks to ASABRContactPlan) and the lexer
    let contact_plan = parse_from_iter::<NoManagement, CMDynStandard, _>(lines).unwrap();

    // Let's use the build helper for convenience
    let mut router = build_generic_router(
        "SpsnHybridParenting",
        contact_plan,
        Some(SpsnOptions {
            check_priority: false,
            check_size: true,
            max_entries: 10,
        }),
    )
    .unwrap();

    // We route a bundle
    let bundle_1 = Bundle {
        source: 0,
        destinations: vec![3],
        priority: 0,
        size: 20.0,
        expiration: 10000.0,
    };

    // let's route with current time == 15
    let out = router
        .route(0, &bundle_1, 15.0, &Vec::new())
        .unwrap()
        .unwrap();
    let (first_hop_contact, route) = out.lazy_get_for_unicast(3).unwrap();

    // Retain a ref to the first_hop manager
    println!("{}", route.borrow());
    // Enqueue the bundle_1
    println!(
        "Enqueueing bundle_1 status : {}",
        first_hop_contact
            .borrow_mut()
            .manager
            .manual_enqueue(&bundle_1)
    );

    // We route a bundle
    let bundle_2 = Bundle {
        source: 0,
        destinations: vec![3],
        priority: 0,
        size: 20.0,
        expiration: 10000.0,
    };

    // let's route with current time == 15, and ensure that the queueing is taken into account
    let out = router
        .route(0, &bundle_2, 15.0, &Vec::new())
        .unwrap()
        .unwrap();
    let (first_hop_contact, route) = out.lazy_get_for_unicast(3).unwrap();
    println!("{}", route.borrow());

    // Enqueue the bundle_2
    println!(
        "Enqueueing bundle_2 status : {}",
        first_hop_contact
            .try_borrow_mut()
            .unwrap()
            .manager
            .manual_enqueue(&bundle_2)
    );
    println!();
    println!(
        "Contact 0 has now 2 bundles in the queue (size: 2 x 20), unless we unqueue manually, the delay will be considered"
    );
    println!();
    // We route a bundle
    let bundle_3 = Bundle {
        source: 0,
        destinations: vec![4],
        priority: 0,
        size: 20.0,
        expiration: 10000.0,
    };
    let out = router.route(0, &bundle_3, 15.0, &Vec::new()).unwrap();
    println!(
        "Sending bundle 3 to node 4, the routing output should be None: {}",
        out.is_none()
    );
    println!();
    println!(
        "Simulate transmission success of bundle_1, Contact 0 should not be a blocker anymore"
    );
    println!(
        "Dequeueing bundle_1, status : {}",
        first_hop_contact
            .borrow_mut()
            .manager
            .manual_dequeue(&bundle_1)
    );
    println!("Retry for bundle 3");
    let out = router
        .route(0, &bundle_3, 15.0, &Vec::new())
        .unwrap()
        .unwrap();
    let (_, route) = out.lazy_get_for_unicast(4).unwrap();
    println!("{}", route.borrow());

    // === OUTPUT ===
    // Running with contact plan location=examples/dijkstra_accuracy/contact_plan_1.cp, and destination node=3

    // Route to node 3 at t=220 with 3 hop(s):
    //         - Reach node 0 at t=15 with 0 hop(s)
    //         - Reach node 1 at t=35 with 1 hop(s)
    //         - Reach node 2 at t=120 with 2 hop(s)
    //         - Reach node 3 at t=220 with 3 hop(s)
    // Enqueueing bundle_1 status : true
    // Route to node 3 at t=240 with 3 hop(s):
    //         - Reach node 0 at t=15 with 0 hop(s)
    //         - Reach node 1 at t=55 with 1 hop(s)
    //         - Reach node 2 at t=120 with 2 hop(s)
    //         - Reach node 3 at t=240 with 3 hop(s)
    // Enqueueing bundle_2 status : true

    // Contact 0 has now 2 bundles in the queue (size: 2 x 20), unless we unqueue manually, the delay will be considered

    // Sending bundle 3 to node 4, the routing output should be None: true

    // Simulate transmission success of bundle_1, Contact 0 should not be a blocker anymore
    // Dequeueing bundle_1, status : true
    // Retry for bundle 3
    // Route to node 4 at t=75 with 2 hop(s):
    //         - Reach node 0 at t=15 with 0 hop(s)
    //         - Reach node 1 at t=55 with 1 hop(s)
    //         - Reach node 4 at t=75 with 2 hop(s)
}
