use a_sabr::{
    contact_manager::legacy::evl::{EVLManager, PBEVLManager},
    contact_plan::{asabr_file_lexer::FileLexer, from_asabr_lexer::ASABRContactPlan},
    node_manager::none::NoManagement,
};

fn main() {
    // Exo 3: parse cp_1 (A-SABR format)
    let cp_1 = "exercises/1-asabr-format-static/contact_plan.asabr";
    // Use the "NoManagement" type for the node managers.
    // Use the "EVLManager" for the contacts managers.

    let mylexer_res = FileLexer::new(cp_1);
    let mut my_lexer = match mylexer_res {
        Ok(val) => val,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    let (nodes, contacts) =
        match ASABRContactPlan::parse::<NoManagement, EVLManager>(&mut my_lexer, None, None) {
            Ok((nodes, contacts)) => (nodes, contacts),
            Err(err) => {
                println!("{}", err);
                return;
            }
        };

    println!("CP_1:\n{:#?}", (&nodes, &contacts));

    // Exo 4: We now want to have PBEVLManager (P for priority and B for budgeted)

    // This approach shows 3 levels of priority and expects a maximum volume for each priority
    // The specific members become <rate> <delay> <maxvol_0> <maxvol_1> <maxvol_2>

    // Modify the file contact_plan_PBEVL.asabr (cp_2), to comply to the PBEVL format
    let cp_2 = "exercises/1-asabr-format-static/contact_plan_PBEVL.asabr";
    // Parse cp_2

    let mylexer_res = FileLexer::new(cp_2);
    let mut mylexer_res_bis = match mylexer_res {
        Ok(val) => val,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    let (nodes_bis, contacts_bis) = match ASABRContactPlan::parse::<NoManagement, PBEVLManager>(
        &mut mylexer_res_bis,
        None,
        None,
    ) {
        Ok((nodes, contacts)) => (nodes, contacts),
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    println!("CP_2:\n{:#?}", (&nodes_bis, &contacts_bis));
}
