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
}
