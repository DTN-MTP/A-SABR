use core::cmp::Ordering;

use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    multigraph::Multigraph,
    node_manager::NodeManager,
    pathfinding::{HybridParentingOrd, destination::FindableDest},
    paths::PathFragment,
};

use super::Distance;

/// A struct allowing to use the Schedule-Aware Bundle Routing distance definition.
///
/// `SABR` is used to implement the `Distance` trait, providing a comparison method
/// for determining the order of `RouteStage` instances based on a set of criteria
/// (such as `at_time` (i.e. arrival time), `hop_count`, and `expiration`).
#[derive(Debug)]
pub struct SABR {}

impl<'id, NM: NodeManager, CM: ContactManager, D: FindableDest<'id, NM, CM>>
    Distance<'id, NM, CM, D> for SABR
{
    /// Compares two `RouteStage` instances to determine their ordering based on
    /// the SABR standard tie-break rules.
    ///
    /// The comparison follows these rules, in descending order of priority:
    /// 1. `at_time`: The `RouteStage` with a later `at_time` is considered greater.
    /// 2. `hop_count`: If `at_time` is equal, the one with a higher `hop_count` is greater.
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
    fn cmp(
        first: &PathFragment<'id>,
        second: &PathFragment<'id>,
        _graph: &Multigraph<'id, NM, CM>,
        _bundle: &Bundle,
        _dest: &D,
    ) -> Ordering {
        super::cmp_by(first, second, |path| {
            (path.recv.end, path.hop_count, path.expiration)
        })
        // TODO: Readd expiration
    }
}

impl<NM: NodeManager, CM: ContactManager> HybridParentingOrd<NM, CM> for SABR {
    #[inline(always)]
    fn keep_both<'id>(
        first: &PathFragment<'id>,
        second: &PathFragment<'id>,
        _graph: &Multigraph<'id, NM, CM>,
        _bundle: &Bundle,
    ) -> bool {
        matches!(
            (
                first.hop_count.cmp(&second.hop_count),
                first.recv.end.cmp(&second.recv.end)
            ),
            (Ordering::Less, Ordering::Greater) | (Ordering::Greater, Ordering::Less)
        )
    }
}
