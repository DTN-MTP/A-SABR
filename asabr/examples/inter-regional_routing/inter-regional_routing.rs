use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use a_sabr::{
    bundle::Bundle,
    contact_plan::asabr_file_lexer::parse_from_iter,
    errors::ASABRError,
    multigraph::{NodeRef, RoutableNodeRef},
    node_manager::none::NoManagement,
    parsing::CMDynStandard,
    pathfinding::top_level::aliases::SpsnHybridParenting,
    utils::{Routing, SingeSourceRouter},
};
use generativity::make_guard;

fn main() -> Result<(), ASABRError> {
    let cp_path = "asabr/examples/inter-regional_routing/asabr_format_dynamic.cp";
    // All nodes will have the same management approach (NoManagement) but the contacts may be of various types.
    // The manager type is Box<dyn ContactManager> through CMDynStandard, selected by contact-plan markers.
    let file = File::open(cp_path).unwrap();
    let lines = BufReader::new(file).lines().map(|l| l.unwrap());

    let contact_plan = parse_from_iter::<NoManagement, CMDynStandard>(lines).unwrap();
    println!(
        "A-SABR CP parsed (statically for nodes, dynamically for contacts), found {} nodes (no management) & {} contacts (of various types)",
        contact_plan.vnodes.len() + contact_plan.realnodes.len(),
        contact_plan.contacts.len()
    );

    make_guard!(id);
    let mut router =
        SingeSourceRouter::<_, _, SpsnHybridParenting<1, _, _, _>, RoutableNodeRef>::build(
            id,
            contact_plan,
            (10, ()),
        )?;

    println!("Virtual nodes:");
    println!("{}", *router);

    println!("\n---\n");

    // We will route a bundle to the virtual gateway node 8.
    let bundle = Bundle {
        priority: 0,
        size: 1,
        expiration: 10000,
    };

    let Ok(NodeRef::I(source)) = router.node_id_ref(0.into()) else {
        panic!("Expected RNodeRef for source node 0")
    };
    router.set_source(source)?;
    let destination = router.node_id_ref(8.into())?.routable().unwrap();

    // We schedule the bundle (resource updates were conducted).
    let out = router.route(destination, 0, &bundle, None)?;

    if let Some((path, _)) = out {
        println!("{}", path);
    }

    Ok(())
}
