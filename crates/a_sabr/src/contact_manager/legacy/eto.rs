use crate::generate_prio_volume_manager;

// With ETO the delay due to the queue is taken into account (from the current time)
// and the updates are not automatic, the queue is expected to be modified by
// external means
generate_prio_volume_manager!(ETOManager, true, false, 1, false);
// with priorities (3 levels)
generate_prio_volume_manager!(PETOManager, true, false, 3, false);
// with priorities (3 levels) and maximum budgets per level
generate_prio_volume_manager!(PBETOManager, true, false, 3, true);
