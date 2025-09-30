use a_sabr::{
    contact_manager::{legacy::evl::EVLManager, seg::SegmentationManager, ContactManager},
    contact_plan::{asabr_file_lexer::FileLexer, from_asabr_lexer::ASABRContactPlan},
    node_manager::none::NoManagement,
    parsing::{coerce_cm, ContactMarkerMap},
};

fn main() {
    // Exo 6: Make the management dynamic

    // The following code will parse the CP statically

    // Change the code and the contact plan to allow dynamic parsing:

    //  "node1" (id 0) is the local node and:
    // - We want the ETOManager for the first hops
    // - We want contact segmentation for the hop between id 2 and id 3.

    // let cp_1 = "exercises/3-asabr-dynamic-parsing/contact_plan.asabr";

    // let mylexer_res = FileLexer::new(cp_1);
    // let mut my_lexer = match mylexer_res {
    //     Ok(val) => val,
    //     Err(err) => {
    //         println!("{}", err);
    //         return;
    //     }
    // };

    // let (nodes, contacts) =
    //     match ASABRContactPlan::parse::<NoManagement, EVLManager>(&mut my_lexer, None, None) {
    //         Ok((nodes, contacts)) => (nodes, contacts),
    //         Err(err) => {
    //             println!("{}", err);
    //             return;
    //         }
    //     };

    // println!("CP_1:\n{:#?}", (&nodes, &contacts));

    // Solution:
    let cp_1 = "exercises/3-asabr-dynamic-parsing/contact_plan.asabr";

    let mylexer_res = FileLexer::new(cp_1);
    let mut my_lexer = match mylexer_res {
        Ok(val) => val,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    let mut contact_dispatch: ContactMarkerMap = ContactMarkerMap::new();
    contact_dispatch.add("eto", coerce_cm::<EVLManager>);
    contact_dispatch.add("seg", coerce_cm::<SegmentationManager>);
    contact_dispatch.add("evl", coerce_cm::<EVLManager>);

    let (nodes, contacts) = match ASABRContactPlan::parse::<NoManagement, Box<dyn ContactManager>>(
        &mut my_lexer,
        None,
        Some(&contact_dispatch),
    ) {
        Ok((nodes, contacts)) => (nodes, contacts),
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    println!("CP_1:\n{:#?}", (&nodes, &contacts));
}
