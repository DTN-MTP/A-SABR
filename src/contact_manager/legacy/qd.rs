use crate::generate_prio_volume_manager;

// With queue delay, the delay due to the queue is taken into account (from the start of the contact)
// and the updates are automatic (we do not "scan" an actual local queue), we increase
// the queue size when we schedule a bundle
generate_prio_volume_manager!(QDManager, true, true, 1, false);
// with priorities (3 levels)
generate_prio_volume_manager!(PQDManager, true, true, 3, false);
// with priorities (3 levels) and maximum budgets per level
generate_prio_volume_manager!(PBQDManager, true, true, 3, false);
