use crate::contact_manager::legacy::LegacyManager;

/// EVL manager without priority or budget handling.
pub type EVLManager = LegacyManager<false, true, 1, 0>;
/// EVL manager with priority handling.
pub type PEVLManager = LegacyManager<false, true, 3, 0>;
/// EVL manager with priority and budget handling.
pub type PBEVLManager = LegacyManager<false, true, 3, 1>;
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
        let before = manager.dry_run_tx(ti, C_START, &bp0(1000)).unwrap();
        for _ in 0..3 {
            let tx_data = manager.dry_run_tx(ti, C_START, &bp0(1000)).unwrap();
            manager.schedule_tx(ti, tx_data, &bp0(1000)).unwrap();
        }

        let after = manager.dry_run_tx(ti, C_START, &bp0(1000)).unwrap();

        assert_eq!(
            before.send.start, after.send.start,
            "TEST FAILED: EVL tx_start should not be affected by queue occupancy."
        );
    }
}
