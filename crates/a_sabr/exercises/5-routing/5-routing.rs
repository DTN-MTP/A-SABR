// use a_sabr::bundle::Bundle;
// use a_sabr::contact_manager::legacy::evl::EVLManager;
// use a_sabr::contact_plan::asabr_file_lexer::FileLexer;
// use a_sabr::contact_plan::from_asabr_lexer::ASABRContactPlan;
// use a_sabr::node_manager::none::NoManagement;
// use a_sabr::routing::aliases::build_generic_router;
// use a_sabr::utils::pretty_print;

fn main() {
    // Exo 8: routing

    // Build a router and schedule a bundle.
    // Suggested tests:
    //     - with contact_plan_1.asabr, and node 3 as the destination
    //     - with contact_plan_2.asabr, and node 4 as the destination
    // For each scenario, test with VolCgrHybridParenting, VolCgrContactParenting, VolCgrHybridParenting

    // let dest = 3;
    // let dest = 4;

    // let algo = "VolCgrNodeParenting";
    // let algo = "VolCgrContactParenting";
    // let algo = "VolCgrHybridParenting";

    // let cp = "exercises/5-routing/contact_plan_1.asabr";
    // let cp = "exercises/5-routing/contact_plan_2.asabr";

    // let mut mylexer = FileLexer::new(cp).unwrap();

    // let (nodes, contacts) =
    //     ASABRContactPlan::parse::<NoManagement, EVLManager>(&mut mylexer, None, None).unwrap();
}
