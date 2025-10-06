use a_sabr::bundle::Bundle;
use a_sabr::contact_manager::legacy::evl::EVLManager;
use a_sabr::contact_plan::asabr_file_lexer::FileLexer;
use a_sabr::contact_plan::from_asabr_lexer::ASABRContactPlan;
use a_sabr::node_manager::none::NoManagement;
use a_sabr::routing::aliases::build_generic_router;
use a_sabr::utils::pretty_print;

fn main() {

    let mut mylexer = FileLexer::new("exercises/5-routing/contact_plan.asabr").unwrap();

    let (nodes, contacts) = ASABRContactPlan::parse::<NoManagement, EVLManager>(
        &mut mylexer,
        None,
        None,
    )
    .unwrap();

    let mut router = build_generic_router::<NoManagement, EVLManager>(
        "VolCgrHybridParenting",
        nodes,
        contacts,
        None,
    );

    // We route a bundle
    let bundle = Bundle {
        source: 0,
        destinations: vec![4],
        priority: 0,
        size: 0.0,
        expiration: 1000.0,
    };

    let out = router.route(0, &bundle, 0.0, &Vec::new()).unwrap();
    let (_first_hop_contact, route) = out.lazy_get_for_unicast(4).unwrap();
    pretty_print(route);
}
