use crate::generate_prio_volume_manager;

// With queue delay, the delay due to the queue is taken into account (from the start of the contact)
// and the updates are automatic (we do not "scan" an actual local queue), we increase
// the queue size when we schedule a bundle
generate_prio_volume_manager!(QDManager, true, true, 1, false);
// with priorities (3 levels)
generate_prio_volume_manager!(PQDManager, true, true, 3, false);
// with priorities (3 levels) and maximum budgets per level
generate_prio_volume_manager!(PBQDManager, true, true, 3, true);

#[cfg(test)]
mod tests {
    use super::{PBQDManager, PQDManager, QDManager};
    use crate::contact_manager::ContactManager;
    use crate::contact_manager::legacy::test_helpers::*;

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
        let contact = make_contact_info(C_START, C_END);

        // 2000 bytes schedulés → queue_size/rate = 2000/1000 = 2s de décalage
        manager
            .schedule_tx(&contact, C_START, &bp0(2000.0))
            .unwrap();

        // at_time=0 < contact_start décalé (2.0) → tx_start doit être 2.0
        let data = manager.dry_run_tx(&contact, C_START, &bp0(100.0)).unwrap();
        assert_eq!(
            data.tx_start, 2.0,
            "TEST FAILED: tx_start should be shifted by queue delay from contact start."
        );
    }

    #[test]
    fn late_arriving_bundle_ignores_queue_shift() {
        let mut manager = qd();
        let contact = make_contact_info(C_START, C_END);

        // queue décale contact_start à 2.0
        manager
            .schedule_tx(&contact, C_START, &bp0(2000.0))
            .unwrap();

        // at_time=5.0 > 2.0 → tx_start doit être at_time, pas le décalage
        let data = manager.dry_run_tx(&contact, 5.0, &bp0(100.0)).unwrap();
        assert_eq!(
            data.tx_start, 5.0,
            "TEST FAILED: tx_start should be at_time when it arrives after the queue shift."
        );
    }

    #[test]
    fn queue_shift_can_push_bundle_past_contact_end() {
        let mut manager = qd();
        let contact = make_contact_info(C_START, C_END);
        manager
            .schedule_tx(&contact, C_START, &bp0(9900.0))
            .unwrap();
        assert!(
            manager.dry_run_tx(&contact, C_START, &bp0(200.0)).is_none(),
            "TEST FAILED: Bundle should not fit when queue shift pushes tx_end past contact end."
        );
    }
}
