use std::cmp::Ordering;

use crate::{contact_manager::ContactManager, route_stage::RouteStage};

pub mod hop;
pub mod sabr;

/// A trait that allows RouteStages to define custom distance comparison strategies.
///
/// # Type Parameters
/// - `CM`: A type that implements the `ContactManager` trait, representing the contact management
///         system used to manage and compare routes.
pub trait Distance<CM>
where
    Self: Sized,
    CM: ContactManager,
{
    /// Compares the distances between two `RouteStage` instances.
    ///
    /// This method provides a total ordering of `RouteStage` instances based on
    /// their distances, returning an `Ordering` (`Less`, `Equal`, or `Greater`)
    /// based on whether the `lhs` route is shorter, equal to, or longer than
    /// the `rhs` route.
    ///
    /// # Parameters
    /// - `lhs`: A reference to the first route stage to compare.
    /// - `rhs`: A reference to the second route stage to compare.
    ///
    /// # Returns
    /// - `Ordering::Less` if `lhs` is shorter than `rhs`.
    /// - `Ordering::Equal` if `lhs` and `rhs` are the same.
    /// - `Ordering::Greater` if `lhs` is longer than `rhs`.
    fn cmp(lhs: &RouteStage<CM>, rhs: &RouteStage<CM>) -> Ordering;

    /// Checks if two `RouteStage` instances are equal in distance.
    ///
    /// This method determines if the distances of `lhs` and `rhs` are equal.
    ///
    /// # Parameters
    /// - `lhs`: A reference to the first route stage to check.
    /// - `rhs`: A reference to the second route stage to check.
    ///
    /// # Returns
    /// - `true` if `lhs` and `rhs` are equal in distance.
    /// - `false` otherwise.
    fn eq(lhs: &RouteStage<CM>, rhs: &RouteStage<CM>) -> bool;
}
