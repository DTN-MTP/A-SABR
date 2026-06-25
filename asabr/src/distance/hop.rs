use core::cmp::Ordering;

use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    multigraph::{Multigraph, RNodeRef},
    node_manager::NodeManager,
    pathfinding::HybridParentingOrd,
    paths::PathFragment,
};

use super::Distance;

/// A struct allowing to use a variant of the Schedule-Aware Bundle Routing distance definition, where
/// a fewer hop count is prioritized over an earlier arrival time.
///
/// `Hop` is used to implement the `Distance` trait, providing a comparison method
/// for determining the order of `RouteStage` instances based on a set of criteria
/// (such as `at_time` (i.e. arrival time), `hop_count`, and `expiration`).
#[derive(Debug)]
pub struct Hop {}

impl<NM: NodeManager, CM: ContactManager> Distance<NM, CM> for Hop {
    /// Compares two `RouteStage` instances to determine their ordering based on
    /// the SABR standard tie-break rules, but by prioritizing fewer hop counts before earliest arrival times.
    ///
    /// The comparison follows these rules, in descending order of priority:
    /// 1. `hop_count`: The `RouteStage` with a higher `hop_count` is considered greater.
    /// 2. `at_time`: If `hop_count` is equal, the one with a later `at_time` is greater.
    /// 3. `expiration`: If both `at_time` and `hop_count` are equal, the one with a lower `expiration` is greater.
    ///
    /// # Parameters
    /// - `first`: The first route stage to compare.
    /// - `second`: The second route stage to compare.
    ///
    /// # Returns
    /// - `Ordering::Greater` if `first` is considered greater than `second` based on the criteria.
    /// - `Ordering::Less` if `second` is considered greater than `first`.
    /// - `Ordering::Equal` if both stages are equal by all criteria.
    ///
    /// # Performance
    /// This function is marked with `#[inline(always)]` for potential performance optimizations.
    #[inline(always)]
    fn cmp<'id>(
        first: &PathFragment<'id>,
        second: &PathFragment<'id>,
        _graph: &Multigraph<'id, NM, CM>,
        _bundle: &Bundle,
    ) -> Ordering {
        super::cmp_by(first, second, |frag| {
            (frag.hop_count, frag.arrival_time.end)
        })
        // TODO: Readd expiration
    }
}

impl<NM: NodeManager, CM: ContactManager> HybridParentingOrd<NM, CM> for Hop {
    #[inline(always)]
    fn keep_both<'id>(
        first: &PathFragment<'id>,
        second: &PathFragment<'id>,
        _graph: &Multigraph<'id, NM, CM>,
        _bundle: &Bundle,
        _actual_node: RNodeRef<'id>,
    ) -> bool {
        match (
            first.hop_count.cmp(&second.hop_count),
            first.arrival_time.end.cmp(&second.arrival_time.end),
        ) {
            (Ordering::Less, Ordering::Greater) | (Ordering::Greater, Ordering::Less) => true,
            _ => false,
        }
    }
}
