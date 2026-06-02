use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use a_sabr::{
    contact_manager::{
        ContactManager,
        legacy::{
            eto::ETOManager,
            evl::{EVLManager, PEVLManager},
            qd::{PQDManager, QDManager},
        },
        segmentation::seg::SegmentationManager,
    },
    contact_plan::{
        asabr_file_lexer::FileLexer, from_asabr_lexer::ASABRContactPlan,
        from_ion_file::IONContactPlan, from_tvgutil_file::TVGUtilContactPlan,
    },
    node_manager::none::NoManagement,
    parsing::{ContactMarkerMap, coerce_cm},
};

fn main() {
    // ION, with contact segmentation
    let file = File::open("examples/contact_plans/ion_format.cp").unwrap();
    let lines: Vec<String> = BufReader::new(file).lines().map(|l| l.unwrap()).collect();

    let contact_plan = IONContactPlan::parse::<NoManagement, SegmentationManager, _>(
        lines.iter().map(|s| s.as_str()),
    )
    .unwrap();
    println!(
        "ION CP parsed, found {} nodes (no management) & {} contacts (segmentation)",
        contact_plan.vertices.len(),
        contact_plan.contacts.len()
    );
    // ION, with EVL
    let file = File::open("examples/contact_plans/ion_format.cp").unwrap();
    let lines: Vec<String> = BufReader::new(file).lines().map(|l| l.unwrap()).collect();

    let contact_plan =
        IONContactPlan::parse::<NoManagement, EVLManager, _>(lines.iter().map(|s| s.as_str()))
            .unwrap();
    println!(
        "ION CP parsed, found {} nodes (no management) & {} contacts (EVL)",
        contact_plan.vertices.len(),
        contact_plan.contacts.len()
    );

    // ION, with EVL + priorities
    let file = File::open("examples/contact_plans/ion_format.cp").unwrap();
    let lines: Vec<String> = BufReader::new(file).lines().map(|l| l.unwrap()).collect();

    let contact_plan =
        IONContactPlan::parse::<NoManagement, PEVLManager, _>(lines.iter().map(|s| s.as_str()))
            .unwrap();
    println!(
        "ION CP parsed, found {} nodes (no management) & {} contacts (EVL with priorities)",
        contact_plan.vertices.len(),
        contact_plan.contacts.len()
    );

    // tvg-util, with contact segmentation
    let file = File::open("examples/contact_plans/tvgutil_format.cp").unwrap();
    let json: serde_json::Value = serde_json::from_reader(file).unwrap();

    let contact_plan =
        TVGUtilContactPlan::parse::<NoManagement, SegmentationManager>(json.clone()).unwrap();
    println!(
        "Tvg-util CP parsed, found {} nodes (no management) & {} contacts (segmentation)",
        contact_plan.vertices.len(),
        contact_plan.contacts.len()
    );

    // tvg-util, with EVL
    let contact_plan = TVGUtilContactPlan::parse::<NoManagement, EVLManager>(json.clone()).unwrap();
    println!(
        "Tvg-util CP parsed, found {} nodes (no management) & {} contacts (EVL)",
        contact_plan.vertices.len(),
        contact_plan.contacts.len()
    );

    // tvg-util, with QD + priorities
    let contact_plan = TVGUtilContactPlan::parse::<NoManagement, PQDManager>(json).unwrap();
    println!(
        "Tvg-util CP parsed, found {} nodes (no management) & {} contacts (queue-delay with priorities)",
        contact_plan.vertices.len(),
        contact_plan.contacts.len()
    );
    let file = File::open("examples/contact_plans/asabr_format_static.cp").unwrap();
    let lines: Vec<String> = BufReader::new(file).lines().map(|l| l.unwrap()).collect();

    let mut mylexer = FileLexer::new(lines.iter().map(|s| s.as_str()));
    let contact_plan =
        ASABRContactPlan::parse::<NoManagement, EVLManager>(&mut mylexer, None, None).unwrap();
    println!(
        "A-SABR CP parsed (statically for nodes & contacts), found {} nodes (no management) & {} contacts (EVL)",
        contact_plan.vertices.len(),
        contact_plan.contacts.len()
    );

    // A new lexer must be initialized
    // The CP format is shared for all legacy contact managers, no CP modification required
    let file = File::open("examples/contact_plans/asabr_format_static.cp").unwrap();
    let lines: Vec<String> = BufReader::new(file).lines().map(|l| l.unwrap()).collect();

    let mut mylexer = FileLexer::new(lines.iter().map(|s| s.as_str()));
    let contact_plan =
        ASABRContactPlan::parse::<NoManagement, QDManager>(&mut mylexer, None, None).unwrap();
    println!(
        "A-SABR CP parsed (statically for nodes & contacts), found {} nodes (no management) & {} contacts (queue-delay)",
        contact_plan.vertices.len(),
        contact_plan.contacts.len()
    );
    let file = File::open("examples/contact_plans/asabr_format_dynamic.cp").unwrap();
    let lines: Vec<String> = BufReader::new(file).lines().map(|l| l.unwrap()).collect();

    let mut mylexer = FileLexer::new(lines.iter().map(|s| s.as_str()));
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
    let contact_plan = ASABRContactPlan::parse::<NoManagement, Box<dyn ContactManager>>(
        &mut mylexer,
        None,
        Some(&contact_dispatch),
    )
    .unwrap();
    println!(
        "A-SABR CP parsed (statically for nodes, dynamically for contacts), found {} nodes (no management) & {} contacts (of various types)",
        contact_plan.vertices.len(),
        contact_plan.contacts.len()
    );
}
