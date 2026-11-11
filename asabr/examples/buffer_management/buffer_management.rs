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

    println!("\nSatellite has a memory capacity of 1000");

    // Bundle 1: retention [0, 70]
    let bundle1 = Bundle {
        priority: 0,
        size: 600,
        expiration: 1000,
    };
    let rx1 = TimeInterval { start: 0, end: 10 };
    let tx1 = TimeInterval { start: 60, end: 70 };

    println!("Reserving memory for bundle 1 (size 600) from T=0 to T=70");

    if satellite_node
        .manager
        .dry_run_retention(&bundle1, rx1, 0.into(), tx1, 2.into())
    {
        satellite_node
            .manager
            .commit(&bundle1, rx1, 0.into(), &[(tx1, 2.into())])
            .unwrap();
        println!("    Success! Peak volume: 600");
    }

    // Bundle 2: retention [20, 90]
    let bundle2 = Bundle {
        priority: 0,
        size: 300,
        expiration: 1000,
    };
    let rx2 = TimeInterval { start: 20, end: 30 };
    let tx2 = TimeInterval { start: 80, end: 90 };

    println!("Reserving memory for bundle 2 (size 300) from T=20 to T=90");

    if satellite_node
        .manager
        .dry_run_retention(&bundle2, rx2, 0.into(), tx2, 2.into())
    {
        satellite_node
            .manager
            .commit(&bundle2, rx2, 0.into(), &[(tx2, 2.into())])
            .unwrap();
        println!("    Success! Peak volume: 900");
    }

    // Bundle 3: retention [40, 60]
    let bundle3 = Bundle {
        priority: 0,
        size: 250,
        expiration: 1000,
    };
    let rx3 = TimeInterval { start: 40, end: 45 };
    let tx3 = TimeInterval { start: 55, end: 60 };

    println!("Reserving memory for bundle 3 (size 250) from T=40 to T=60");

    let can_retain_3 =
        satellite_node
            .manager
            .dry_run_retention(&bundle3, rx3, 0.into(), tx3, 2.into());
    println!(
        "    Answer: {} (Insufficient capacity, rejected)",
        can_retain_3
    );

    // === OUTPUT ===
    // A-SABR CP parsed, found 3 nodes (BufferManager) & 2 contacts (ETO)
    //
    // Satellite has a memory capacity of 1000.
    // Reserving memory for bundle 1 (size 600) from T=0 to T=70.
    //     Success! Peak volume: 600
    // Reserving memory for bundle 2 (size 300) from T=20 to T=90.
    //     Success! Peak volume: 900
    // Reserving memory for bundle 3 (size 250) from T=40 to T=60.
    //     Answer: false (Insufficient capacity, rejected)
}
