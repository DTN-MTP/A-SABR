use crate::generate_prio_volume_manager;

// With EVL, the delay due to the queue is not taken into account
// and the updates are automatic (we do not "scan" an actual local queue),
// we just reduce the volume available
generate_prio_volume_manager!(EVLManager, false, true, 1);
generate_prio_volume_manager!(PEVLManager, false, true, 3);
