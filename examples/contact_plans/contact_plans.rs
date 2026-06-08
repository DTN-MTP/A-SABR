use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use a_sabr::{
    contact_manager::{
        legacy::{
            evl::{EVLManager, PEVLManager},
            qd::{PQDManager, QDManager},
        },
        segmentation::seg::SegmentationManager,
    },
    contact_plan::{
        asabr_file_lexer::parse_from_iter, from_ion_file::IONContactPlan,
        from_tvgutil_file::TVGUtilContactPlan,
    },
    node_manager::none::NoManagement,
    parsing::CMDynStandard,
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

    let file = File::open("examples/contact_plans/asabr_format_static.cp").unwrap();
    let lines = BufReader::new(file).lines().map(|l| l.unwrap());
    let contact_plan = parse_from_iter::<_, _, NoManagement, EVLManager>(lines).unwrap();
    println!(
        "A-SABR CP parsed (statically for nodes & contacts), found {} nodes (no management) & {} contacts (EVL)",
        contact_plan.vertices.len(),
        contact_plan.contacts.len()
    );

    // A new lexer must be initialized
    // The CP format is shared for all legacy contact managers, no CP modification required
    let file = File::open("examples/contact_plans/asabr_format_static.cp").unwrap();
    let lines = BufReader::new(file).lines().map(|l| l.unwrap());
    let contact_plan = parse_from_iter::<_, _, NoManagement, QDManager>(lines).unwrap();
    println!(
        "A-SABR CP parsed (statically for nodes & contacts), found {} nodes (no management) & {} contacts (queue-delay)",
        contact_plan.vertices.len(),
        contact_plan.contacts.len()
    );
    let file = File::open("examples/contact_plans/asabr_format_dynamic.cp").unwrap();
    let lines: Vec<String> = BufReader::new(file).lines().map(|l| l.unwrap()).collect();

    let file = File::open("examples/contact_plans/asabr_format_dynamic.cp").unwrap();
    let lines = BufReader::new(file).lines().map(|l| l.unwrap());
    // The manager type should be Box<dyn ContactManager>> (heap allocated, dynamically dispatched)
    // Replace None with a dispatching map for the contact_marker_map argument
    let contact_plan = parse_from_iter::<_, _, NoManagement, CMDynStandard>(lines).unwrap();
    println!(
        "A-SABR CP parsed (statically for nodes, dynamically for contacts), found {} nodes (no management) & {} contacts (of various types)",
        contact_plan.vertices.len(),
        contact_plan.contacts.len()
    );
}
