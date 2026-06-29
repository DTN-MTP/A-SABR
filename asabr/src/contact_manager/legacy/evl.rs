use crate::contact_manager::legacy::LegacyManager;

// With EVL, the delay due to the queue is not taken into account
// and the updates are automatic (we do not "scan" an actual local queue),
// we just reduce the volume available
pub type EVLManager = LegacyManager<false, true, 1, false>;
pub type PEVLManager = LegacyManager<false, true, 3, false>;
pub type PBEVLManager = LegacyManager<false, true, 3, true>;
#[cfg(test)]
mod tests {
    use super::{EVLManager, PBEVLManager, PEVLManager};
    use crate::contact_manager::ContactManager;
    use crate::contact_manager::legacy::test_helpers::*;
    use crate::types::TimeInterval;

    fn evl() -> EVLManager {
        let mut manager = EVLManager::new(RATE, DELAY);
        manager.try_init(&make_contact_info(C_START, C_END));
        manager
    }
    fn pevl() -> PEVLManager {
        let mut manager = PEVLManager::new(RATE, DELAY);
        manager.try_init(&make_contact_info(C_START, C_END));
        manager
    }
    fn pbevl() -> PBEVLManager {
        let mut manager = PBEVLManager::new(RATE, DELAY, [BUDGET_P0, BUDGET_P1, BUDGET_P2]);
        manager.try_init(&make_contact_info(C_START, C_END));
        manager
    }

    crate::generate_common_tests!(evl, EVLManager);
    crate::generate_auto_update_tests!(evl, pevl);
    crate::generate_budget_tests!(pbevl);
    crate::generate_budget_auto_update_tests!(pbevl);

    #[test]
    fn tx_start_unaffected_by_queue_occupancy() {
        let mut manager = evl();
        let ti = TimeInterval {
            start: C_START,
            end: C_END,
        };
        let before = manager.dry_run_tx(ti, C_START, &bp0(1000.0)).unwrap();
        manager.schedule_tx(ti, C_START, &bp0(1000.0)).unwrap();
        manager.schedule_tx(ti, C_START, &bp0(1000.0)).unwrap();
        manager.schedule_tx(ti, C_START, &bp0(1000.0)).unwrap();

        let after = manager.dry_run_tx(ti, C_START, &bp0(1000.0)).unwrap();

        assert_eq!(
            before.tx_window.start, after.tx_window.start,
            "TEST FAILED: EVL tx_start should not be affected by queue occupancy."
        );
    }
}
