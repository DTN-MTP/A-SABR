use a_sabr::bundle::Bundle;
use a_sabr::contact_manager::legacy::eto::ETOManager;
use a_sabr::contact_manager::legacy::evl::EVLManager;
use a_sabr::contact_manager::legacy::qd::QDManager;
use a_sabr::contact_manager::ContactManager;
use a_sabr::contact_plan::asabr_file_lexer::FileLexer;
use a_sabr::contact_plan::from_asabr_lexer::ASABRContactPlan;
use a_sabr::node_manager::none::NoManagement;
use a_sabr::parsing::coerce_cm;
use a_sabr::parsing::ContactMarkerMap;
use a_sabr::routing::aliases::build_generic_router;
use a_sabr::routing::aliases::SpsnOptions;
use a_sabr::utils::pretty_print;

fn main() {
    #[cfg(not(feature = "manual_queueing"))]
    panic!("Please enable the 'manual_queueing' feature.");

    // We want variations for contact management, register ETO and EVL
    let mut contact_dispatch: ContactMarkerMap = ContactMarkerMap::new();
    contact_dispatch.add("eto", coerce_cm::<ETOManager>);
    contact_dispatch.add("evl", coerce_cm::<EVLManager>);
    contact_dispatch.add("qd", coerce_cm::<QDManager>);

    // We create a lexer to retrieve tokens from a file
    let mut mylexer = FileLexer::new("examples/eto_management/contact_plan_1.cp").unwrap();

    // We parse the contact plan (A-SABR format thanks to ASABRContactPlan) and the lexer
    let (nodes, contacts) = ASABRContactPlan::parse::<NoManagement, Box<dyn ContactManager>>(
        &mut mylexer,
        None,
        Some(&contact_dispatch),
    )
    .unwrap();

    // Let's use the build helper for convenience
    let mut router = build_generic_router::<NoManagement, Box<dyn ContactManager>>(
        "SpsnHybridParenting",
        nodes,
        contacts,
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

    pretty_print(route);
    // Enqueue the bundle_1
    #[cfg(feature = "manual_queueing")]
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
    pretty_print(route);

    // Enqueue the bundle_2
    #[cfg(feature = "manual_queueing")]
    println!(
        "Enqueueing bundle_2 status : {}",
        first_hop_contact
            .try_borrow_mut()
            .unwrap()
            .manager
            .manual_enqueue(&bundle_2)
    );
    println!();
    println!("Contact 0 has now 2 bundles in the queue (size: 2 x 20), unless we unqueue manually, the delay will be considered");
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
    #[cfg(feature = "manual_queueing")]
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
    pretty_print(route);

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
