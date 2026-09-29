use crate::multigraph::{ContactRef, RealNodeRef};
use crate::types::{Date, HopCount, TimeInterval};
use core::fmt::Display;

/// Represents an intermediate hop in a route, typically used for multi-hop communication or routing.
///
/// This struct encapsulates the `Contact` and parent `RouteStage` information necessary to move from
/// one stage to the next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViaHop<'id> {
    /// A reference to the contact for this hop, representing the intermediate node.
    pub contact: ContactRef<'id>,
    /// A reference to the parent route stage for this hop.
    pub parent_frag: usize,
    /// Transmission interval used by this hop.
    pub send: TimeInterval,
}

/// Represents the end of a path to a node.
/// The rest of the path is available through via
#[derive(derivative::Derivative, Copy, Clone, PartialEq, Eq)]
#[derivative(Debug)]
pub struct PathFragment<'id> {
    // /// A flag that indicates if this path is disabled.
    // pub is_disabled: bool,
    /// An optional `ViaHop` that stores information about the previous hops in the path.
    pub via: Option<ViaHop<'id>>,
    /// The number of hops taken to reach this stage from the source.
    pub hop_count: HopCount,

    /// The arrival time to the final node in the original disktra
    pub recv: TimeInterval,
    /// A reference to the receiving node for this hop.
    pub rx_node: RealNodeRef<'id>,
    /// An approximation of this route expiration (min of contacts lifetime end).
    pub expiration: Date,
}

impl<'id> PathFragment<'id> {
    /// Creates a new `PathFragment` with the specified parameters.
    ///
    /// # Parameters
    ///
    /// * `arrival_time` - The arrival interval at the receiving node.
    /// * `via_hop` - Optional previous-hop information.
    /// * `hop_count` - Number of hops from the source.
    /// * `rx_node` - Receiving node for this fragment.
    ///
    /// # Returns
    ///
    /// A new instance of `PathFragment`.
    pub fn new(
        recv: TimeInterval,
        via_hop: Option<ViaHop<'id>>,
        hop_count: HopCount,
        rx_node: RealNodeRef<'id>,
        expiration: Date,
    ) -> Self {
        Self {
            recv,
            via: via_hop,
            hop_count,
            rx_node,
            // cumulative_delay: 0.0,
            expiration,
        }
    }
    /// Creates the initial path fragment at the source node.
    pub fn new_start(time: Date, node: RealNodeRef<'id>) -> Self {
        Self {
            via: None,
            hop_count: 0,
            recv: TimeInterval {
                start: time,
                end: time,
            },
            rx_node: node,
            expiration: Date::MAX,
        }
    }
}

impl<'id> Display for PathFragment<'id> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(
            f,
            "Route arriving during t={} with {} hop(s), passing by {:#?}",
            self.recv, self.hop_count, self.via
        )
    }
}
