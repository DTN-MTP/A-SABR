use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;

use a_sabr::bundle::Bundle;
use a_sabr::contact_manager::ContactManager;
use a_sabr::contact_plan::asabr_file_lexer::parse_from_iter;
use a_sabr::errors::ASABRError;
use a_sabr::multigraph::RoutableNodeRef;
use a_sabr::node_manager::none::NoManagement;
use a_sabr::parsing::CMDynStandard;
use a_sabr::pathfinding::top_level::aliases::SpsnHybridParenting;
use a_sabr::utils::{Routing, SingeSourceRouter};
use generativity::make_guard;

fn main() -> Result<(), ASABRError> {
    // We read the file content
    let file = File::open("asabr/examples/eto_management/contact_plan_1.cp").unwrap();
    let lines = BufReader::new(file).lines().map(|l| l.unwrap());

    // We parse the contact plan
    let contact_plan = parse_from_iter(lines).unwrap();

    // Securely initialize the Multigraph lifecycle
    make_guard!(id);

    // We want variations for contact management, register ETO and EVL
    let mut router = SingeSourceRouter::<
        NoManagement,
        CMDynStandard,
        SpsnHybridParenting<1, _, _, _>,
        RoutableNodeRef,
    >::build(id, contact_plan, (10, ()))?;

    // Retrieve typed references for nodes 0 and 3
    // Validity guarantees are given now, reducing later checks
    let src_0 = router.node_id_ref(0.into()).unwrap().internal().unwrap();
    router.set_source(src_0)?;
    let dest_3 = router.node_id_ref(3.into()).unwrap().routable().unwrap();

    // Scenario 1: Route the first bundle to node 3
    let bundle_1 = Bundle {
        priority: 0,
        size: 20,
        expiration: 10000,
    };

    // let's route with current time == 15
    let (iter_1, first) = router.route(dest_3, 15, &bundle_1, None)?.unwrap();

    println!("{}", iter_1);

    // Explicitly enqueue the bundle to simulate transmission delay
    let first_hop_contact = first.via.as_ref().unwrap().contact;
    println!(
        "Enqueueing bundle_1 status : {}",
        router[first_hop_contact].manager.manual_enqueue(&bundle_1)
    );

    // Scenario 2: Route a second bundle to node 3

    let bundle_2 = Bundle {
        priority: 0,
        size: 20,
        expiration: 10000,
    };

    // let's route with current time == 15, and ensure that the queueing is taken into account
    let (iter_2, first) = router.route(dest_3, 15, &bundle_2, None)?.unwrap();

    println!("{}", iter_2);

    let first_hop_contact_2 = first.via.as_ref().unwrap().contact;

    // Enqueue the bundle_2
    println!(
        "Enqueueing bundle_2 status : {}",
        router[first_hop_contact_2]
            .manager
            .manual_enqueue(&bundle_2)
    );

    println!();
    println!(
        "Contact 0 has now 2 bundles in the queue (size: 2 x 20), unless we unqueue manually, the delay will be considered"
    );
    println!();

    let bundle_3 = Bundle {
        priority: 0,
        size: 20,
        expiration: 10000,
    };
    let dest_4 = router.node_id_ref(4.into()).unwrap().routable().unwrap();

    // Should fail as the transmission queue is full
    let out_3 = router.route(dest_4, 15, &bundle_3, None)?;
    println!(
        "Sending bundle 3 to node 4, the routing output should be None: {}",
        out_3.is_none()
    );

    println!();
    println!(
        "Simulate transmission success of bundle_1, Contact 0 should not be a blocker anymore"
    );

    // Free queue space and retry routing
    println!(
        "Dequeueing bundle_1, status : {}",
        router[first_hop_contact].manager.manual_dequeue(&bundle_1)
    );

    println!("Retry for bundle 3");

    let (iter_4, _first) = router.route(dest_4, 15, &bundle_3, None)?.unwrap();
    println!("{}", iter_4);

    // === OUTPUT ===
    // Running with contact plan location=asabr/examples/dijkstra_accuracy/contact_plan_1.cp, and destination node=3

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

    Ok(())
}
