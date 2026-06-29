//! With queue delay, the delay due to the queue is taken into account (from the start of the contact)
//! and the updates are automatic (we do not "scan" an actual local queue), we increase
//! the queue size when we schedule a bundle

use crate::contact_manager::legacy::LegacyManager;

pub type QDManager = LegacyManager<true,true,1,false>;
pub type PQDManager = LegacyManager<true,true,3,false>;
pub type PBQDManager = LegacyManager<true,true,3,true>;


#[cfg(test)]
mod tests {
    use super::{PBQDManager, PQDManager, QDManager};
    use crate::contact_manager::ContactManager;
    use crate::contact_manager::legacy::test_helpers::*;
use crate::types::TimeInterval;

    fn qd() -> QDManager {
        let mut manager = QDManager::new(RATE, DELAY);
        manager.try_init(&make_contact_info(C_START, C_END));
        manager
    }
    fn pqd() -> PQDManager {
        let mut manager = PQDManager::new(RATE, DELAY);
        manager.try_init(&make_contact_info(C_START, C_END));
        manager
    }
    fn pbqd() -> PBQDManager {
        let mut manager = PBQDManager::new(RATE, DELAY, [BUDGET_P0, BUDGET_P1, BUDGET_P2]);
        manager.try_init(&make_contact_info(C_START, C_END));
        manager
    }

    crate::generate_common_tests!(qd, QDManager);
    crate::generate_auto_update_tests!(qd, pqd);
    crate::generate_budget_tests!(pbqd);
    crate::generate_budget_auto_update_tests!(pbqd);

    #[test]
    fn queue_delay_shifts_tx_start_from_contact_start() {
        let mut manager = qd();
        let ti = TimeInterval {
            start: C_START,
            end: C_END,
        };

        manager
            .schedule_tx(ti, C_START, &bp0(2000.0))
            .unwrap();

        let data = manager.dry_run_tx(ti, C_START, &bp0(100.0)).unwrap();
        assert_eq!(
            data.tx_window.start, 2,
            "TEST FAILED: tx_start should be shifted by queue delay from contact start."
        );
    }

    #[test]
    fn late_arriving_bundle_ignores_queue_shift() {
        let mut manager = qd();
        let ti = TimeInterval {
            start: C_START,
            end: C_END,
        };

        manager
            .schedule_tx(ti, C_START, &bp0(2000.0))
            .unwrap();

        let data = manager.dry_run_tx(ti, 5, &bp0(100.0)).unwrap();
        assert_eq!(
            data.tx_window.start, 5,
            "TEST FAILED: tx_start should be at_time when it arrives after the queue shift."
        );
    }

    #[test]
    fn queue_shift_can_push_bundle_past_contact_end() {
        let mut manager = qd();
        let ti = TimeInterval {
            start: C_START,
            end: C_END,
        };
        manager
            .schedule_tx(ti, C_START, &bp0(9900.0))
            .unwrap();
        assert!(
            manager.dry_run_tx(ti, C_START, &bp0(200.0)).is_none(),
            "TEST FAILED: Bundle should not fit when queue shift pushes tx_end past contact end."
        );
    }
}
