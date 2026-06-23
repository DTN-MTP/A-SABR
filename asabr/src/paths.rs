use crate::multigraph::{ContactRef, RNodeRef};
use crate::types::{Date, HopCount, TimeInterval};
use core::fmt::Display;

/// Represents an intermediate hop in a route, typically used for multi-hop communication or routing.
///
/// This struct encapsulates the `Contact` and parent `RouteStage` information necessary to move from
/// one stage to the next.
#[derive(Debug, Clone, Copy)]
pub struct ViaHop<'id> {
    /// A reference to the contact for this hop, representing the intermediate node.
    pub contact: ContactRef<'id>,
    /// A reference to the parent route stage for this hop.
    pub parent_frag: usize,
    /// A reference to the transmitting node for this hop.
    pub tx_node: RNodeRef<'id>,
}

/// Represents the end of a path to a Node.
/// The rest of the path is available through via
///
///  # Type Parameters
/// - `CM`: A type implementing the `ContactManager` trait, responsible for managing the
///   contact's operations.
/// - `NM`: A type implementing the `NodeManager` trait, responsible for managing the
///   node's operations.
#[derive(derivative::Derivative, Copy, Clone)]
#[derivative(Debug)]
pub struct PathFragment<'id> {
    // /// A flag that indicates if this path is disabled.
    // pub is_disabled: bool,
    /// An optional `ViaHop` that stores information about the previous hops in the path.
    pub via: Option<ViaHop<'id>>,
    /// The number of hops taken to reach this stage from the source.
    pub hop_count: HopCount,

    /// The arrival time to the final node in the original disktra
    pub arrival_time: TimeInterval,
    // /// The cumulative transmission delay incurred on this path, often used for routing optimizations.
    // pub cumulative_delay: Duration,
    // /// The time at which this route stage expires, indicating when it is no longer valid.
    // pub expiration: Date,
}

impl<'id> PathFragment<'id> {
    /// Creates a new `RouteStage` with the specified parameters.
    ///
    /// # Parameters
    ///
    /// * `at_time` - The time at which this route stage is scheduled.
    /// * `to_node` - The destination node ID.
    /// * `via_hop` - An optional ViaHop information.
    ///
    /// # Returns
    ///
    /// A new instance of `RouteStage`.
    pub fn new(
        arrival_time: TimeInterval,
        via_hop: Option<ViaHop<'id>>,
        hop_count: HopCount,
    ) -> Self {
        Self {
            arrival_time,
            // is_disabled: false,
            via: via_hop,
            hop_count,
            // cumulative_delay: 0.0,
            // expiration: Date::MAX,
        }
    }
    pub fn new_start(time: Date) -> Self {
        Self {
            via: None,
            hop_count: 0,
            arrival_time: TimeInterval {
                start: time,
                end: time,
            },
        }
    }
}

impl<'id> Display for PathFragment<'id> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(
            f,
            "Route arriving during t={} with {} hop(s), passing by {:#?}",
            self.arrival_time, self.hop_count, self.via
        )
    }
}
