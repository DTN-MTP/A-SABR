use crate::types::{Date, NodeID, Priority, Volume};

/// A structure representing a routing bundle containing essential information for pathfinding.
///
/// The `Bundle` struct encapsulates the routing details required for determining optimal paths
/// in a network, including source and destination nodes, priority, size, and expiration time.
#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Clone)]
pub struct Bundle {
    /// The starting node identifier for the routing operation.
    pub source: NodeID,
    ///  A vector of node identifiers representing the target destinations for the routing operation.
    pub destinations: Vec<NodeID>,
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
    /// might have been skipped if a bundle of lower size needs to be scheduled for a tree originally
    /// computed for a larger bundle. `self` is the bundle attached to the tree.
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
        // lower volume paths might have been skiped
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
