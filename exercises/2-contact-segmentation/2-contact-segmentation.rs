use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use a_sabr::{
    contact_manager::segmentation::seg::SegmentationManager,
    contact_plan::{asabr_file_lexer::FileLexer, from_asabr_lexer::ASABRContactPlan},
    node_manager::none::NoManagement,
};

fn main() {
    // Exo 5:
    // Here is the code to parse an A-SABR cp for contact segmentation
    // The cp is however incomplete, go check it out

    let cp_1 = "exercises/2-contact-segmentation/contact_plan.asabr";
    let file = File::open(cp_1).unwrap();
    let iter: Vec<_> = BufReader::new(file).lines().map(|s| s.unwrap()).collect();

    let mut my_lexer = FileLexer::new(iter.iter().map(|s| s.as_str()));

    let contact_plan = match ASABRContactPlan::parse::<NoManagement, SegmentationManager>(
        &mut my_lexer,
        None,
        None,
    ) {
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
