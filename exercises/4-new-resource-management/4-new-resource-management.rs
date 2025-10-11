use a_sabr::{
    bundle::Bundle,
    contact::ContactInfo,
    contact_manager::{ContactManager, ContactManagerTxData},
    contact_plan::{asabr_file_lexer::FileLexer, from_asabr_lexer::ASABRContactPlan},
    node_manager::none::NoManagement,
    parsing::{DispatchParser, Lexer, Parser, ParsingState},
    types::{DataRate, Date, Duration, Token, Volume},
};

// Exo 7: Implementation of a new volume management technique.

// The objective is to re-implement EVL (Effective Volume Limit)
// - Define de structure rate, delay, residual_volume (the EVL)
// - Implement the ContactManager trait:
//      - Leave "todo!()" in the core of the get_original_volume if you compile with --all-features)
//      - Do not implement manual_enqueue & manual_dequeue
//      - try_init will allow the correct initialization of "residual_volume"
//      - for dry_run_tx, calculating intermediary variables like tx_start, tx_time, and tx_end may help.
//      - schedule_tx can reuse dry_run_tx
// - Implement the Parser<MyEVL> trait:
//      - parse a rate
//      - parse a delay

// #[derive(Debug)]
// struct MyEVL {
//     // todo
// }

fn main() {
    //     let cp_1 = "exercises/4-new-resource-management/contact_plan.asabr";

    //     let mylexer_res = FileLexer::new(cp_1);
    //     let mut my_lexer = match mylexer_res {
    //         Ok(val) => val,
    //         Err(err) => {
    //             println!("{}", err);
    //             return;
    //         }
    //     };

    //     let (nodes, contacts) =
    //         match ASABRContactPlan::parse::<NoManagement, MyEVL>(&mut my_lexer, None, None) {
    //             Ok((nodes, contacts)) => (nodes, contacts),
    //             Err(err) => {
    //                 println!("{}", err);
    //                 return;
    //             }
    //         };

    //     println!("CP:\n{:#?}", (&nodes, &contacts));
}
