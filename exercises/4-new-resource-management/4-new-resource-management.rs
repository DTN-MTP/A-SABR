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

#[derive(Debug)]
struct MyEVL {
    rate: DataRate,
    delay: Duration,
    residual_volume: Volume,
}

impl ContactManager for MyEVL {
    fn dry_run_tx(
        &self,
        contact_data: &ContactInfo,
        at_time: Date,
        bundle: &Bundle,
    ) -> Option<ContactManagerTxData> {
        if bundle.size > self.residual_volume {
            return None;
        }

        let tx_start: Date = if contact_data.start > at_time {
            contact_data.start
        } else {
            at_time
        };
        let tx_time = bundle.size / self.rate;
        let tx_end = tx_start + tx_time;
        if tx_time > contact_data.end {
            return None;
        }
        return Some(ContactManagerTxData {
            tx_start,
            tx_end,
            delay: self.delay,
            expiration: contact_data.end,
            arrival: contact_data.end + self.delay,
        });
    }

    fn schedule_tx(
        &mut self,
        contact_data: &ContactInfo,
        at_time: Date,
        bundle: &Bundle,
    ) -> Option<ContactManagerTxData> {
        self.residual_volume -= bundle.size;
        return self.dry_run_tx(contact_data, at_time, bundle);
    }

    fn try_init(&mut self, contact_data: &ContactInfo) -> bool {
        if self.delay < 0.0 || self.rate < 0.0 {
            return false;
        }

        self.residual_volume = (contact_data.end - contact_data.start) * self.rate;
        return true;
    }

    #[cfg(feature = "first_depleted")]
    fn get_original_volume(&self) -> Volume {
        todo!()
    }
}

impl Parser<MyEVL> for MyEVL {
    fn parse(lexer: &mut dyn Lexer) -> ParsingState<MyEVL> {
        let delay: Duration;
        let rate: DataRate;

        let rate_state = <DataRate as Token<DataRate>>::parse(lexer);
        match rate_state {
            ParsingState::Finished(value) => rate = value,
            ParsingState::Error(msg) => return ParsingState::Error(msg),
            ParsingState::EOF => {
                return ParsingState::Error(format!(
                    "Parsing failed ({})",
                    lexer.get_current_position()
                ))
            }
        }

        let delay_state = <Duration as Token<Duration>>::parse(lexer);
        match delay_state {
            ParsingState::Finished(value) => delay = value,
            ParsingState::Error(msg) => return ParsingState::Error(msg),
            ParsingState::EOF => {
                return ParsingState::Error(format!(
                    "Parsing failed ({})",
                    lexer.get_current_position()
                ))
            }
        }
        return ParsingState::Finished(MyEVL {
            rate,
            delay,
            residual_volume: 0.0,
        });
    }
}

impl DispatchParser<MyEVL> for MyEVL {}

fn main() {
    let cp_1 = "exercises/4-new-resource-management/contact_plan.asabr";

    let mylexer_res = FileLexer::new(cp_1);
    let mut my_lexer = match mylexer_res {
        Ok(val) => val,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    let (nodes, contacts) =
        match ASABRContactPlan::parse::<NoManagement, MyEVL>(&mut my_lexer, None, None) {
            Ok((nodes, contacts)) => (nodes, contacts),
            Err(err) => {
                println!("{}", err);
                return;
            }
        };

    println!("CP:\n{:#?}", (&nodes, &contacts));
}
