use crate::types::{Date, NodeID, Priority, Volume};
extern crate alloc;
/// A routing bundle containing the information needed for pathfinding.
///
/// The `Bundle` stores the source node, priority, volume, and expiration time.
/// Destinations are handled separately by the pathfinding API.
#[derive(Clone, Debug)]
pub struct Bundle {
    /// The starting node identifier for the routing operation.
    pub source: NodeID,
    /// The priority level of the bundle, used to influence routing decisions.
    pub priority: Priority,
    /// The volume size associated with the bundle, which can affect routing constraints.
    pub size: Volume,
    /// The expiration date for the bundle.
    pub expiration: Date,
}

impl Bundle {
    /// Determines if the current bundle "shadows" existing routes based on size and priority checks.
    ///
    /// This method is used to enhance volume-aware pathfinding by tracking possible paths that
    /// might have been skipped if a bundle of lower size or lower priority needs to be scheduled for a tree originally
    /// computed for a larger or higher priority bundle. `self` is the bundle attached to the tree.
    ///
    /// # Parameters
    ///
    /// * `other` - The other bundle to compare against.
    /// * `check_by_size` - A boolean indicating whether to consider size in the comparison.
    /// * `check_by_priority` - A boolean indicating whether to consider priority in the comparison.
    ///
    /// # Returns
    ///
    /// * `bool` - Returns `true` if the current bundle shadows the other bundle, otherwise `false`.
    pub fn shadows(&self, other: &Bundle, check_by_size: bool, check_by_priority: bool) -> bool {
        // lower volume paths might have been skipped
        if check_by_size && self.size > other.size {
            return true;
        }

        // a higher priority volume can claim lower priority volume congested paths
        if check_by_priority && self.priority > other.priority {
            return true;
        }
        false
    }
}
