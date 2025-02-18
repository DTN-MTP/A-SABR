

use a_sabr::node_manager::NodeManager;

struct NoRetention {}

impl NodeManager for NoRetention {

    fn dry_run_tx(&self, waiting_since: a_sabr::types::Date, start: a_sabr::types::Date, end: a_sabr::types::Date, bundle: &a_sabr::bundle::Bundle) -> bool {
        todo!()
    }


    fn schedule_tx(&mut self, waiting_since: a_sabr::types::Date, start: a_sabr::types::Date, end: a_sabr::types::Date, bundle: &a_sabr::bundle::Bundle)
        -> bool {
        todo!()
    }

}


fn main() {
    let a = NoRetention{};
}