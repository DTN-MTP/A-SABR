use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;

use a_sabr::bundle::Bundle;
use a_sabr::contact_manager::legacy::eto::ETOManager;
use a_sabr::contact_plan::RealNode;
use a_sabr::contact_plan::asabr_file_lexer::parse_from_iter;
use a_sabr::node_manager::NodeManager;
use a_sabr::node_manager::buffer::BufferManager;
use a_sabr::types::TimeInterval;

fn main() {
    let cp = File::open("examples/buffer_management/buffer_format.asabr").unwrap();
    let lines = BufReader::new(cp).lines().map(|l| l.unwrap());

    let contact_plan = parse_from_iter::<BufferManager, ETOManager>(lines).unwrap();

    println!(
        "A-SABR CP parsed, found {} nodes (BufferManager) & {} contacts (ETO)",
        contact_plan.vnodes.len() + contact_plan.realnodes.len(),
        contact_plan.contacts.len()
    );

    // Retrieve the Satellite node (node 1), which has a capacity of 1000
    let mut satellite_node = match &contact_plan.realnodes[1] {
        RealNode::Inode(node) => node.clone(),
        RealNode::Enode(node) => node.clone(),
    };

    // bundle of size 600 arrives at the satellite
    let bundle = Bundle {
        priority: 0,
        size: 600,
        expiration: 1000,
    };

    println!("\nScenario: Satellite has a memory capacity of 1000.");
    println!(
        "Routing algorithm tries to reserve memory for a bundle of size {}.",
        bundle.size
    );

    let reception_interval = TimeInterval { start: 0, end: 50 };
    let transmission_interval = TimeInterval {
        start: 50,
        end: 100,
    };

    let can_retain = satellite_node.manager.dry_run_retention(
        &bundle,
        reception_interval,
        0.into(),
        transmission_interval,
        2.into(),
    );

    println!(
        "Can the satellite retain the bundle from T=0 to T=100? {}",
        can_retain
    );

    if can_retain {
        let transmissions = vec![(transmission_interval, 2.into())];
        satellite_node
            .manager
            .commit(&bundle, reception_interval, 0.into(), &transmissions)
            .unwrap();
        println!("Memory committed successfully!");
    }

    // Try to send another bundle that exceeds remaining capacity
    let large_bundle = Bundle {
        priority: 0,
        size: 500, // 600 + 500 = 1100 (> 1000)
        expiration: 1000,
    };

    let can_retain_large = satellite_node.manager.dry_run_retention(
        &large_bundle,
        reception_interval,
        0.into(),
        transmission_interval,
        2.into(),
    );

    println!(
        "\nCan the satellite retain a second bundle of size {} in the exact same time window?",
        large_bundle.size
    );
    println!(
        "Answer: {} (Insufficient capacity, safely rejected)",
        can_retain_large
    );

    // === OUTPUT ===
    // A-SABR CP parsed, found 3 nodes (BufferManager) & 2 contacts (ETO)
    //
    // Scenario: Satellite has a memory capacity of 1000.
    // Routing algorithm tries to reserve memory for a bundle of size 600.
    // Can the satellite retain the bundle from T=0 to T=100? true
    // Memory committed successfully!
    //
    // Can the satellite retain a second bundle of size 500 in the exact same time window?
    // Answer: false (Insufficient capacity, safely rejected)
}
