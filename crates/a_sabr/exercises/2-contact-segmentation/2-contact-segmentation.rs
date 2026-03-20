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

    let mylexer_res = FileLexer::new(cp_1);
    let mut my_lexer = match mylexer_res {
        Ok(val) => val,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    let contact_plan = match ASABRContactPlan::parse::<NoManagement, SegmentationManager>(
        &mut my_lexer,
        None,
        None,
    ) {
        Ok(contact_plan) => contact_plan,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    println!("CP:\n{:#?}", (&contact_plan.nodes, &contact_plan.contacts));
}
