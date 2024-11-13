use std::cmp::Ordering;

use crate::{contact_manager::ContactManager, route_stage::RouteStage};

use super::Distance;

/// A struct allowing to use the Schedule-Aware Bundle Routing distance definition.
///
/// `SABR` is used to implement the `Distance` trait, providing a comparison method
/// for determining the order of `RouteStage` instances based on a set of criteria
/// (such as `at_time` (i.e. arrival time), `hop_count`, and `expiration`).
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct SABR {}

impl<CM: ContactManager> Distance<CM> for SABR {
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
    fn cmp(first: &RouteStage<CM, Self>, second: &RouteStage<CM, Self>) -> Ordering {
        if first.at_time > second.at_time {
            return Ordering::Greater;
        } else if first.at_time < second.at_time {
            return Ordering::Less;
        } else if first.hop_count > second.hop_count {
            return Ordering::Greater;
        } else if first.hop_count < second.hop_count {
            return Ordering::Less;
        } else if first.expiration < second.expiration {
            return Ordering::Greater;
        } else if first.expiration > second.expiration {
            return Ordering::Less;
        }
        Ordering::Equal
    }

    /// Partially compares two `RouteStage` instances by delegating to the `cmp` method.
    ///
    /// This function provides a partial ordering between `first` and `second`, allowing for an
    /// `Option<Ordering>` return type that includes `None` in case a comparison cannot be made.
    /// Here, it directly calls `cmp` to perform the comparison and wraps the result in `Some`.
    ///
    /// # Parameters
    /// - `first`: The first route stage to partially compare.
    /// - `second`: The second route stage to partially compare.
    ///
    /// # Returns
    /// - `Some(Ordering)` based on the comparison of `first` and `second`.
    /// - `None` if the comparison is not well-defined (not applicable here as `cmp` is always defined).
    ///
    /// # Performance
    /// This function is marked with `#[inline(always)]` for potential performance optimizations.
    #[inline(always)]
    fn partial_cmp(
        first: &RouteStage<CM, Self>,
        second: &RouteStage<CM, Self>,
    ) -> Option<Ordering> {
        Some(first.cmp(second))
    }

    /// Checks if two `RouteStage` instances are equal based on specific criteria.
    ///
    /// Equality is determined by the following criteria:
    /// - `at_time`: Both instances must have the same `at_time`.
    /// - `hop_count`: Both instances must have the same `hop_count`.
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
    fn eq(first: &RouteStage<CM, Self>, second: &RouteStage<CM, Self>) -> bool {
        first.at_time == second.at_time
            && first.hop_count == second.hop_count
            && first.expiration == second.expiration
    }
}