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
}
