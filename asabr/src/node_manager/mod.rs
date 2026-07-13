extern crate alloc;

use crate::{
    bundle::Bundle,
    errors::ASABRError,
    types::{Date, NodeID, TimeInterval},
};
/// Node manager implementation that applies no resource-management constraints.
pub mod none;

/// Defines node-level resource management and scheduling behavior.
///
/// A `NodeManager` decides whether a bundle can be accepted by a node,
/// how long it must be retained before retransmission, whether candidate
/// transmissions are feasible, and how node resources are updated once a
/// routing decision is committed.
///
/// # Expected guarantees
///
/// Implementations are expected to keep the dry-run and commit methods coherent:
///
/// - If `dry_run_retention(..., transmission, next)` returns `true`, then
///   `dry_run_multi(..., &[(transmission, next)])` should also accept the same
///   transmission, assuming the other parameters are identical.
/// - If `dry_run_multi(...)` accepts a set of transmissions, then `commit(...)`
///   with the same parameters should succeed.
/// - `delay(...)` should not return a date earlier than the reception end time.
///
/// Violating these guarantees can lead to incorrect routing behavior or panics,
/// but should not cause memory unsafety.
///
/// # Simulation methods
///
/// - `accept(bundle, time, sender)` checks whether the node can receive the bundle.
/// - `delay(bundle, reception, sender, next_vertex)` returns the earliest
///   retransmission time.
/// - `dry_run_retention(bundle, reception, sender, transmission, next_vertex)`
///   checks whether one candidate retention/transmission is feasible.
/// - `dry_run_multi(bundle, reception, sender, transmissions)` checks how many
///   candidate transmissions can be accepted.
///
/// # Commit method
///
/// - `commit(bundle, reception, sender, transmissions)` updates node resources
///   after a routing decision has been accepted.
pub trait NodeManager {
    // This is important for optimisation, so no default implementation is provided
    /// Returns `false` if the node cannot receive the bundle during the given interval.
    fn accept(&self, bundle: &Bundle, time: TimeInterval, sender: NodeID) -> bool;

    #[allow(unused_variables)]
    /// Returns the earliest date at which the bundle may be retransmitted.
    ///
    /// The returned date should account for both reception-side and
    /// transmission-side delays.
    ///
    /// `next_vertex` may identify a virtual node.
    fn delay(
        &self,
        bundle: &Bundle,
        reception: TimeInterval,
        sender: NodeID,
        next_vertex: NodeID,
    ) -> Date {
        reception.end
    }

    #[allow(unused_variables)]
    /// Checks whether retaining the bundle until the candidate transmission is allowed.
    ///
    /// This is used during pathfinding to test possible routes without updating
    /// node resources.
    ///
    /// `next_vertex` may identify a virtual node.
    fn dry_run_retention(
        &self,
        bundle: &Bundle,
        reception: TimeInterval,
        sender: NodeID,
        transmission: TimeInterval,
        next_vertex: NodeID,
    ) -> bool;

    /// Simulates accepting the bundle and retransmitting it through multiple contacts.
    ///
    /// Returns `None` if the node cannot accept the bundle. Returns `Some(n)` if
    /// the node can accept the bundle and retransmit it through the first `n`
    /// entries of `transmissions`.
    ///
    /// `transmissions` may contain several entries for multicast-like forwarding,
    /// or be empty when no retransmission is required.
    ///
    /// # Expected guarantees
    ///
    /// This method should be less restrictive than `dry_run_retention(...)`, but
    /// more restrictive than `commit(...)`.
    fn dry_run_multi(
        &self,
        bundle: &Bundle,
        reception: TimeInterval,
        sender: NodeID,
        transmissions: &[(TimeInterval, NodeID)],
    ) -> Option<usize>;

    /// Commits the accepted transmissions and updates this node's resources.
    ///
    /// `transmissions` may contain several entries for multicast-like forwarding,
    /// or be empty when no retransmission is required.
    ///
    /// # Expected guarantee
    ///
    /// This method should not return an error if a previous call to
    /// `dry_run_multi(...)` accepted the same transmissions.
    fn commit(
        &mut self,
        bundle: &Bundle,
        reception: TimeInterval,
        sender: NodeID,
        transmissions: &[(TimeInterval, NodeID)],
    ) -> Result<(), ASABRError>;
}

// Implementation of `NodeManager` for dyn references.
impl<T: AsRef<dyn NodeManager> + AsMut<dyn NodeManager>> NodeManager for T {
    fn accept(&self, bundle: &Bundle, time: TimeInterval, sender: NodeID) -> bool {
        self.as_ref().accept(bundle, time, sender)
    }

    fn delay(
        &self,
        bundle: &Bundle,
        reception: TimeInterval,
        sender: NodeID,
        next: NodeID,
    ) -> Date {
        self.as_ref().delay(bundle, reception, sender, next)
    }

    fn dry_run_retention(
        &self,
        bundle: &Bundle,
        reception: TimeInterval,
        sender: NodeID,
        transmition: TimeInterval,
        next: NodeID,
    ) -> bool {
        self.as_ref()
            .dry_run_retention(bundle, reception, sender, transmition, next)
    }

    fn dry_run_multi(
        &self,
        bundle: &Bundle,
        reception: TimeInterval,
        sender: NodeID,
        transmitions: &[(TimeInterval, NodeID)],
    ) -> Option<usize> {
        self.as_ref()
            .dry_run_multi(bundle, reception, sender, transmitions)
    }

    fn commit(
        &mut self,
        bundle: &Bundle,
        reception: TimeInterval,
        sender: NodeID,
        transmitions: &[(TimeInterval, NodeID)],
    ) -> Result<(), ASABRError> {
        self.as_mut()
            .commit(bundle, reception, sender, transmitions)
    }
}
/// Auto implement NodeManager for wrapper struct where element 0 is the actual node manager
#[macro_export]
macro_rules! transparent_NM {
    ($T:ty) => {
        impl NodeManager for $T {
            fn accept(&self, bundle: &Bundle, time: TimeInterval, sender: NodeID) -> bool {
                self.0.accept(bundle, time, sender)
            }

            fn delay(
                &self,
                bundle: &Bundle,
                reception: TimeInterval,
                sender: NodeID,
                next: NodeID,
            ) -> Date {
                self.0.delay(bundle, reception, sender, next)
            }

            fn dry_run_retention(
                &self,
                bundle: &Bundle,
                reception: TimeInterval,
                sender: NodeID,
                transmition: TimeInterval,
                next: NodeID,
            ) -> bool {
                self.0
                    .dry_run_retention(bundle, reception, sender, transmition, next)
            }

            fn dry_run_multi(
                &self,
                bundle: &Bundle,
                reception: TimeInterval,
                sender: NodeID,
                transmitions: &[(TimeInterval, NodeID)],
            ) -> Option<usize> {
                self.0
                    .dry_run_multi(bundle, reception, sender, transmitions)
            }

            fn commit(
                &mut self,
                bundle: &Bundle,
                reception: TimeInterval,
                sender: NodeID,
                transmitions: &[(TimeInterval, NodeID)],
            ) -> Result<(), ASABRError> {
                self.0.commit(bundle, reception, sender, transmitions)
            }
        }
    };
}
