use crate::generate_prio_volume_manager;

// With ETO the delay due to the queue is taken into account (from the current time)
// and the updates are not automatic, the queue is expected to be modified by
// external means
generate_prio_volume_manager!(ETOManager, true, false, 1, false);
// with priorities (3 levels)
generate_prio_volume_manager!(PETOManager, true, false, 3, false);
// with priorities (3 levels) and maximum budgets per level
generate_prio_volume_manager!(PBETOManager, true, false, 3, true);

#[cfg(test)]
mod tests {
    use super::{ETOManager, PBETOManager, PETOManager};
    use crate::contact_manager::ContactManager;
    use crate::contact_manager::legacy::test_helpers::*;

    fn eto() -> ETOManager {
        let mut manager = ETOManager::new(RATE, DELAY);
        manager.try_init(&make_contact_info(C_START, C_END));
        manager
    }
    fn _peto() -> PETOManager {
        let mut manager = PETOManager::new(RATE, DELAY);
        manager.try_init(&make_contact_info(C_START, C_END));
        manager
    }
    fn pbeto() -> PBETOManager {
        let mut manager = PBETOManager::new(RATE, DELAY, [BUDGET_P0, BUDGET_P1, BUDGET_P2]);
        manager.try_init(&make_contact_info(C_START, C_END));
        manager
    }

    crate::generate_common_tests!(eto, ETOManager);
    crate::generate_budget_tests!(pbeto);

    #[test]
    fn schedule_tx_does_not_consume_volume() {
        let mut manager = eto();
        let contact = make_contact_info(C_START, C_END);
        for i in 0..20 {
            assert!(
                manager
                    .schedule_tx(&contact, C_START, &bp0(1000.0))
                    .is_some(),
                "TEST FAILED: ETO schedule_tx should never saturate (call {}).",
                i + 1
            );
        }
    }

    #[test]
    fn schedule_tx_always_returns_same_result() {
        let mut manager = eto();
        let contact = make_contact_info(C_START, C_END);
        let bundle = bp0(1000.0);
        let first = manager.schedule_tx(&contact, C_START, &bundle);
        let second = manager.schedule_tx(&contact, C_START, &bundle);

        assert_eq!(
            first, second,
            "TEST FAILED: ETO schedule_tx should return identical results since queue is never updated."
        );
    }

    #[cfg(feature = "manual_queueing")]
    #[test]
    fn manual_enqueue_shifts_tx_start_from_at_time() {
        let mut manager = eto();
        let contact = make_contact_info(C_START, C_END);
        manager.manual_enqueue(&bp0(2000.0));
        let data = manager.dry_run_tx(&contact, 3.0, &bp0(100.0)).unwrap();
        assert_eq!(
            data.tx_start, 5.0,
            "TEST FAILED: tx_start should be at_time + queue/rate for ETO."
        );
    }

    #[cfg(feature = "manual_queueing")]
    #[test]
    fn manual_enqueue_shift_can_push_past_contact_end() {
        let mut manager = eto();
        let contact = make_contact_info(C_START, C_END);
        manager.manual_enqueue(&bp0(9900.0));
        assert!(
            manager.dry_run_tx(&contact, C_START, &bp0(200.0)).is_none(),
            "TEST FAILED: Bundle should not fit when manual queue shift pushes tx_end past contact end."
        );
    }
}
