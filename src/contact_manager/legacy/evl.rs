use crate::generate_prio_volume_manager;

// With EVL, the delay due to the queue is not taken into account
// and the updates are automatic (we do not "scan" an actual local queue),
// we just reduce the volume available
generate_prio_volume_manager!(EVLManager, false, true, 1, false);
// with priorities (3 levels)
generate_prio_volume_manager!(PEVLManager, false, true, 3, false);
// with priorities (3 levels) and maximum budgets per level
generate_prio_volume_manager!(PBEVLManager, false, true, 3, true);

#[cfg(test)]
mod tests {
    use super::{EVLManager, PBEVLManager, PEVLManager};
    use crate::contact_manager::ContactManager;
    use crate::contact_manager::legacy::test_helpers::*;

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
}
