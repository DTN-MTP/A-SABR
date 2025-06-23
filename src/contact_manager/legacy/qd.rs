use crate::{generate_basic_volume_manager, generate_prio_volume_manager};

// With queue delay, the delay due to the queue is taken into account
// and the updates are automatic (we do not "scan" an actual local queue), we increase
// the queue size when we schedule a bundle
generate_basic_volume_manager!(QDManager, true, true);
generate_prio_volume_manager!(PQDManager, true, true, 3);
