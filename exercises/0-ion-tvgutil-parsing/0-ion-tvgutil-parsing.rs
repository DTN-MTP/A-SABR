// use a_sabr::{
//     contact_manager::legacy::{evl::EVLManager, qd::QDManager},
//     contact_plan::{from_ion_file::IONContactPlan, from_tvgutil_file::TVGUtilContactPlan},
//     node_manager::none::NoManagement,
// };

fn main() {
    // Exo 1: retrieve and display the contacts from the examples/0-ion-tvgutil-parsing/contact_plan.ion file

    // Use that
    // let cp_ion = "exercises/0-ion-tvgutil-parsing/contact_plan.ion";

    // Use the "NoManagement" type for the node managers.
    // Use the "EVLManager" for the contacts managers. (EVL as defined in SABR)
    // You can also try with "QDManager" and "ETOManager",
    // or their priority enabled versions "PEVLManager", "PQDManger", etc.

    // Display Nodes and Contacts with the {:?} (standard) or {:#?} (pettry print) formats
    // Example: println!("{:?}", node);

    // cargo run --example 0-ion-tvgutil-parsing  # Expected to fail
    // cargo run --example 0-ion-tvgutil-parsing --features=debug
    // cargo run --example 0-ion-tvgutil-parsing --features=debug,contact_work_area
    // cargo run --example 0-ion-tvgutil-parsing --features=debug,contact_suppression,first_depleted

    // Those different compilation options should already be reflected in the standard output of the contacts
}
