use std::cmp::Ordering;

use crate::{
    contact_manager::ContactManager, node_manager::NodeManager,
    pathfinding::hybrid_parenting::HybridParentingOrd, route_stage::RouteStage,
};

use super::Distance;

/// A struct allowing to use a varient of the Schedule-Aware Bundle Routing distance definition, where
/// a fewer hop count is prioritized over an earlier arrival time.
///
/// `Hop` is used to implement the `Distance` trait, providing a comparison method
/// for determining the order of `RouteStage` instances based on a set of criteria
/// (such as `at_time` (i.e. arrival time), `hop_count`, and `expiration`).
#[cfg_attr(feature = "debug", derive(Debug))]
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
    fn cmp(first: &RouteStage<NM, CM>, second: &RouteStage<NM, CM>) -> Ordering {
        if first.hop_count > second.hop_count {
            return Ordering::Greater;
        } else if first.hop_count < second.hop_count {
            return Ordering::Less;
        } else if first.at_time > second.at_time {
            return Ordering::Greater;
        } else if first.at_time < second.at_time {
            return Ordering::Less;
        } else if first.expiration < second.expiration {
            return Ordering::Greater;
        } else if first.expiration > second.expiration {
            return Ordering::Less;
        }
        Ordering::Equal
    }

    /// Checks if two `RouteStage` instances are equal based on specific criteria.
    ///
    /// Equality is determined by the following criteria:
    /// - `hop_count`: Both instances must have the same `hop_count`.
    /// - `at_time`: Both instances must have the same `at_time`.
    /// - `expiration`: Both instances must have the same `expiration`..
    ///
    /// # Parameters
    /// - `first`: The first route stage to check for equality.
    /// - `second`: The second route stage to check for equality.
    ///
    /// # Returns
    /// - `true` if `first` and `second` meet the criteria for equality.
    /// - `false` otherwise.
    ///
    /// # Performance
    /// This function is marked with `#[inline(always)]` for potential performance optimizations.
    #[inline(always)]
    fn eq(first: &RouteStage<NM, CM>, second: &RouteStage<NM, CM>) -> bool {
        first.at_time == second.at_time
            && first.hop_count == second.hop_count
            && first.expiration == second.expiration
    }
}

impl<NM: NodeManager, CM: ContactManager> HybridParentingOrd<NM, CM> for Hop {
    // For Hop, the secondary metric to consider is the arrival time.
    fn can_retain(prop: &RouteStage<NM, CM>, known: &RouteStage<NM, CM>) -> bool {
        return prop.at_time < known.at_time;
    }
    // Ignore expiration constraints to prioritize performance.
    fn must_prune(prop: &RouteStage<NM, CM>, known: &RouteStage<NM, CM>) -> bool {
        return prop.at_time <= known.at_time && prop.hop_count <= known.hop_count;
    }
}
