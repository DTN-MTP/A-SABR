// use a_sabr::{
//     contact_manager::legacy::evl::{EVLManager, PBEVLManager},
//     contact_plan::{asabr_file_lexer::FileLexer, from_asabr_lexer::ASABRContactPlan},
//     node_manager::none::NoManagement,
// };

use a_sabr::{contact_manager::legacy::evl::{EVLManager, PBEVLManager}, contact_plan::{asabr_file_lexer::FileLexer, from_asabr_lexer::ASABRContactPlan}, node_manager::none::NoManagement};

fn main() {
    // Exo 3: parse cp_1 (A-SABR format)
    let cp_1 = "exercises/1-asabr-static-parsing/contact_plan.asabr";
    let mut lexer = FileLexer::new(cp_1).expect("Lexer failed");
    // Use the "NoManagement" type for the node managers.
    // Use the "EVLManager" for the contacts managers.
    let nodes_cp1 = ASABRContactPlan::parse::<NoManagement, EVLManager>(&mut lexer,None,None).expect("Parsing failed");
    println!("EVLManager parsing: {:#?}",nodes_cp1);
    
    // Exo 4: We now want to have PBEVLManager (P for priority and B for budgeted)

    // This approach shows 3 levels of priority and expects a maximum volume for each priority
    // The specific members become <rate> <delay> <maxvol_0> <maxvol_1> <maxvol_2>

    // Modify the file contact_plan_PBEVL.asabr (cp_2), to comply to the PBEVL format
    let cp_2 = "exercises/1-asabr-static-parsing/contact_plan_PBEVL.asabr";
    let mut lexer2 = FileLexer::new(cp_2).expect("Lexer failed");
    let nodes_cp2 = ASABRContactPlan::parse::<NoManagement,PBEVLManager>(&mut lexer2,None,None).expect("Parsing failed");
    println!("PBEVLManager parsing: {:#?}",nodes_cp2);
    // Parse cp_2
}
