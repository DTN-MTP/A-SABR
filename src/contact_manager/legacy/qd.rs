use crate::generate_prio_volume_manager;

// With queue delay, the delay due to the queue is taken into account
// and the updates are automatic (we do not "scan" an actual local queue), we increase
// the queue size when we schedule a bundle
generate_prio_volume_manager!(QDManager, false, true, 1);
generate_prio_volume_manager!(PQDManager, true, true, 3);
