use a_sabr::{
    contact_manager::{
        legacy::{
            eto::ETOManager,
            evl::{EVLManager, PEVLManager},
            qd::{PQDManager, QDManager},
        },
        seg::SegmentationManager,
        ContactManager,
    },
    contact_plan::{
        asabr_file_lexer::FileLexer, from_asabr_lexer::ASABRContactPlan,
        from_ion_file::IONContactPlan, from_tvgutil_file::TVGUtilContactPlan,
    },
    node_manager::none::NoManagement,
    parsing::{coerce_cm, ContactMarkerMap},
};

fn main() {
    // ION, with contact segmentation
    let (nodes, contacts) = IONContactPlan::parse::<NoManagement, SegmentationManager>(
        "examples/contact_plans/ion_format.cp",
    )
    .unwrap();
    println!(
        "ION CP parsed, found {} nodes (no management) & {} contacts (segmentation)",
        nodes.len(),
        contacts.len()
    );
    // ION, with EVL
    let (nodes, contacts) =
        IONContactPlan::parse::<NoManagement, EVLManager>("examples/contact_plans/ion_format.cp")
            .unwrap();
    println!(
        "ION CP parsed, found {} nodes (no management) & {} contacts (EVL)",
        nodes.len(),
        contacts.len()
    );

    // ION, with EVL + priorities
    let (nodes, contacts) =
        IONContactPlan::parse::<NoManagement, PEVLManager>("examples/contact_plans/ion_format.cp")
            .unwrap();
    println!(
        "ION CP parsed, found {} nodes (no management) & {} contacts (EVL with priorities)",
        nodes.len(),
        contacts.len()
    );

    // tvg-util, with contact segmentation
    let (nodes, contacts) = TVGUtilContactPlan::parse::<NoManagement, SegmentationManager>(
        "examples/contact_plans/tvgutil_format.cp",
    )
    .unwrap();
    println!(
        "Tvg-util CP parsed, found {} nodes (no management) & {} contacts (segmentation)",
        nodes.len(),
        contacts.len()
    );

    // tvg-util, with EVL
    let (nodes, contacts) = TVGUtilContactPlan::parse::<NoManagement, EVLManager>(
        "examples/contact_plans/tvgutil_format.cp",
    )
    .unwrap();
    println!(
        "Tvg-util CP parsed, found {} nodes (no management) & {} contacts (EVL)",
        nodes.len(),
        contacts.len()
    );

    // tvg-util, with QD + priorities
    let (nodes, contacts) = TVGUtilContactPlan::parse::<NoManagement, PQDManager>(
        "examples/contact_plans/tvgutil_format.cp",
    )
    .unwrap();
    println!("Tvg-util CP parsed, found {} nodes (no management) & {} contacts (queue-delay with priorities)", nodes.len(), contacts.len());

    let mut mylexer = FileLexer::new("examples/contact_plans/asabr_format_static.cp").unwrap();
    let (nodes, contacts) =
        ASABRContactPlan::parse::<NoManagement, EVLManager>(&mut mylexer, None, None).unwrap();
    println!("A-SABR CP parsed (statically for nodes & contacts), found {} nodes (no management) & {} contacts (EVL)", nodes.len(), contacts.len());

    // A new lexer must be initialized
    // The CP format is shared for all legacy contact managers, no CP modification required
    let mut mylexer = FileLexer::new("examples/contact_plans/asabr_format_static.cp").unwrap();
    let (nodes, contacts) =
        ASABRContactPlan::parse::<NoManagement, QDManager>(&mut mylexer, None, None).unwrap();
    println!("A-SABR CP parsed (statically for nodes & contacts), found {} nodes (no management) & {} contacts (queue-delay)", nodes.len(), contacts.len());

    let mut mylexer = FileLexer::new("examples/contact_plans/asabr_format_dynamic.cp").unwrap();
    // All nodes will have the same management approach (NoManagement) but the contacts may be of various types
    // We provide a map with markers that will allow the parser to create the correct contacts types thanks to
    // the markers provides in the contact plan
    let mut contact_dispatch: ContactMarkerMap = ContactMarkerMap::new();
    contact_dispatch.add("eto", coerce_cm::<ETOManager>);
    contact_dispatch.add("qd", coerce_cm::<QDManager>);
    contact_dispatch.add("evl", coerce_cm::<EVLManager>);
    contact_dispatch.add("seg", coerce_cm::<SegmentationManager>);

    // The manager type should be Box<dyn ContactManager>> (heap allocated, dynamically dispatched)
    // Replace None with a dispatching map for the contact_marker_map argument
    let (nodes, contacts) = ASABRContactPlan::parse::<NoManagement, Box<dyn ContactManager>>(
        &mut mylexer,
        None,
        Some(&contact_dispatch),
    )
    .unwrap();
    println!("A-SABR CP parsed (statically for nodes, dynamically for contacts), found {} nodes (no management) & {} contacts (of various types)", nodes.len(), contacts.len());
}
