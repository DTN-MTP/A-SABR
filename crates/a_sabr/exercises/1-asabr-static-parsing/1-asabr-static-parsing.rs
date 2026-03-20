// use a_sabr::{
//     contact_manager::legacy::evl::{EVLManager, PBEVLManager},
//     contact_plan::{asabr_file_lexer::FileLexer, from_asabr_lexer::ASABRContactPlan},
//     node_manager::none::NoManagement,
// };

fn main() {
    // Exo 3: parse cp_1 (A-SABR format)
    // let cp_1 = "exercises/1-asabr-format-static/contact_plan.asabr";
    // Use the "NoManagement" type for the node managers.
    // Use the "EVLManager" for the contacts managers.

    // Exo 4: We now want to have PBEVLManager (P for priority and B for budgeted)

    // This approach shows 3 levels of priority and expects a maximum volume for each priority
    // The specific members become <rate> <delay> <maxvol_0> <maxvol_1> <maxvol_2>

    // Modify the file contact_plan_PBEVL.asabr (cp_2), to comply to the PBEVL format
    // let cp_2 = "exercises/1-asabr-format-static/contact_plan_PBEVL.asabr";
    // Parse cp_2
}
