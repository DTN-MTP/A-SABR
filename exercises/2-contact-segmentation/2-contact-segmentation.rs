use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use a_sabr::{
    contact_manager::segmentation::seg::SegmentationManager,
    contact_plan::asabr_file_lexer::parse_from_iter, node_manager::none::NoManagement,
};

fn main() {
    // Exo 5:
    // Here is the code to parse an A-SABR cp for contact segmentation
    // The cp is however incomplete, go check it out

    let cp_1 = "exercises/2-contact-segmentation/contact_plan.asabr";
    let file = File::open(cp_1).unwrap();
    let iter = BufReader::new(file).lines().map(|s| s.unwrap());

    let contact_plan = match parse_from_iter::<_, _, NoManagement, SegmentationManager>(iter) {
        Ok(contact_plan) => contact_plan,
        Err(err) => {
            println!("{err}");
            return;
        }
    };

    println!(
        "CP:\n{:#?}",
        (&contact_plan.vertices, &contact_plan.contacts)
    );
}
