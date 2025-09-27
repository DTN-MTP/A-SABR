use a_sabr::{
    contact_manager::legacy::{evl::EVLManager, qd::QDManager},
    contact_plan::{from_ion_file::IONContactPlan, from_tvgutil_file::TVGUtilContactPlan},
    node_manager::none::NoManagement,
};

fn main() {
    // Exo 1: retrieve and display the contacts from the examples/0-parsing/contact_plan.ion file

    // Use the "NoManagement" type for the NM managers.
    // Use the "EVLManager" for the contacts managers. (EVL as defined in SABR)

    // Display Nodes and Contacts is the {:?} or {:#?} formats
    // Example: println!("{:#?}", node);

    // cargo run --example 0-parsing  # Expected to fail
    // cargo run --example 0-parsing --features=debug
    // cargo run --example 0-parsing --features=debug,contact_work_area
    // cargo run --example 0-parsing --features=debug,contact_suppression,first_depleted

    // Solution:
    let (nodes, contacts) = match IONContactPlan::parse::<NoManagement, EVLManager>(
        "exercises/0-parsing/contact_plan.ion",
    ) {
        Ok((nodes, contacts)) => (nodes, contacts),
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    println!("ION CP:\n{:#?}", (&nodes, &contacts));

    // Exo 2: retrieve and display the contacts from the examples/0-parsing/contact_plan.tvgutil file

    // Use the "NoManagement" type for the NM managers.
    // Use the "QDManager" for the contacts managers. (queue-delay by Carlo Caini)

    // Solution:
    let (nodes, contacts) = match TVGUtilContactPlan::parse::<NoManagement, QDManager>(
        "exercises/0-parsing/contact_plan.tvgutil",
    ) {
        Ok((nodes, contacts)) => (nodes, contacts),
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    println!("TVG CP:\n{:#?}", (&nodes, &contacts));
}
