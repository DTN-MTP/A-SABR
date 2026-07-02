extern crate alloc;

use core::cmp::Ordering;

use crate::{
    bundle::Bundle, contact_manager::ContactManager, multigraph::Multigraph,
    node_manager::NodeManager, paths::PathFragment,
};

pub mod hop;
pub mod prio_queue;
pub mod sabr;

/// A trait that allows RouteStages to define custom distance comparison strategies.
///
/// # Type Parameters
/// - `NM`: A type that implements the `NodeManager` trait.
/// - `CM`: A type that implements the `ContactManager` trait, representing the contact management
///   system used to manage and compare routes.
pub trait Distance<NM, CM>
where
    Self: Sized,
    NM: NodeManager,
    CM: ContactManager,
{
    /// Compares the distances between two `RouteStage` instances.
    ///
    /// This method provides a total ordering of `RouteStage` instances based on
    /// their distances, returning an `Ordering` (`Less`, `Equal`, or `Greater`)
    /// based on whether the `first` route is shorter, equal to, or longer than
    /// the `second` route.
    ///
    /// # Parameters
    /// - `first`: The first route stage to compare.
    /// - `second`: The second route stage to compare.
    ///
    /// # Returns
    /// - `Ordering::Less` if `first` is shorter than `second`.
    /// - `Ordering::Equal` if `first` and `second` are the same.
    /// - `Ordering::Greater` if `first` is longer than `second`.
    fn cmp<'id>(
        first: &PathFragment<'id>,
        second: &PathFragment<'id>,
        graph: &Multigraph<'id, NM, CM>,
        bundle: &Bundle,
    ) -> Ordering;
}

pub fn cmp_by<T, F: Fn(T) -> O, O: Ord>(a: T, b: T, f: F) -> Ordering {
    f(a).cmp(&f(b))
}
