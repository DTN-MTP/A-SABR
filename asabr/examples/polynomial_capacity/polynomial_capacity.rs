use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;

use a_sabr::bundle::Bundle;
use a_sabr::contact_manager::segmentation::poly_seg::PolySegManager;
use a_sabr::contact_plan::asabr_file_lexer::parse_from_iter;
use a_sabr::node_manager::none::NoManagement;

fn main() {
    let file = File::open("asabr/examples/polynomial_capacity/poly_format.cp").unwrap();
    let lines = BufReader::new(file).lines().map(|l| l.unwrap());

    // We parse the contact plan statically using PolySegManager
    let contact_plan = parse_from_iter::<NoManagement, PolySegManager<26>>(lines).unwrap();

    println!(
        "A-SABR CP parsed, found {} nodes (no management) & {} contacts (polynomial capacity)",
        contact_plan.vnodes.len() + contact_plan.realnodes.len(),
        contact_plan.contacts.len()
    );

    // Retrieve the first contact (Earth to Mars)
    let earth_mars_contact = contact_plan.contacts[0].clone();

    // Scenario 1: Route a bundle through the polynomial capacity contact
    // Analytically, if P(t) = 10 + 3t^2, F(t) = 10t + t^3. For t=4, F(4) = 40 + 64 = 104.
    let bundle = Bundle {
        priority: 1,
        size: 104,
        expiration: 1000,
    };

    println!();
    println!("Sending a bundle of size {} at T=0...", bundle.size);
    println!("Reminder: The flow rate follows the curve P(t) = 10 + 3t^2");

    // Execute the capacity computation (via integer dichotomy)
    if let Some(tx_data) = earth_mars_contact.0.dry_run_tx(0.into(), &bundle) {
        println!();
        println!("Transmission successful!");
        println!("  - Transmission start : T = {}", tx_data.send.start);
        println!(
            "  - Transmission end   : T = {} (Exact dichotomy)",
            tx_data.send.end
        );
        println!(
            "  - Arrival on Mars    : T = {} (Tx end + Network delay)",
            tx_data.recv.end
        );
    } else {
        println!();
        println!("Transmission failed: Contact does not have enough capacity.");
    }

    // === OUTPUT ===
    // A-SABR CP parsed, found 2 nodes (no management) & 1 contacts (polynomial capacity)
    //
    // Sending a bundle of size 104 at T=0...
    // Reminder: The flow rate follows the curve P(t) = 10 + 3t^2
    //
    // Transmission successful!
    //   - Transmission start : T = 0
    //   - Transmission end   : T = 4 (Exact dichotomy)
    //   - Arrival on Mars    : T = 8 (Tx end + Network delay)
}