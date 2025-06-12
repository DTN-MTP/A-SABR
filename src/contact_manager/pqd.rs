use crate::generate_basic_volume_manager_with_priority;

// With queue delay, the delay due to the queue is taken into account
// and the updates are automatic (we do not "scan" an actual local queue), we increase
// the queue size when we schedule a bundle
generate_basic_volume_manager_with_priority!(PQDManager, true, true);
