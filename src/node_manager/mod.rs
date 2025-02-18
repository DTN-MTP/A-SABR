use crate::{bundle::Bundle, types::Date};
use std::any::Any;

pub mod none;

/// A trait for managing and scheduling operations on nodes in a network.
///
/// The `NodeManager` trait defines methods for dry-run (simulation) and actual scheduling
/// of processing, transmission (tx), and reception (rx) of a `Bundle` at specified times.
/// This trait is useful for implementing custom logic for nodes that need to manage bundle
/// processing and data transfer in a time-dependent manner.
pub trait NodeManager {
    /// Simulates processing a `Bundle` at a specified time.
    ///
    /// This method performs a dry run to estimate the processing time of a bundle without
    /// actually executing the process. It returns the estimated completion time.
    ///
    /// # Parameters
    /// - `at_time`: The time at which the dry-run process simulation should start.
    /// - `bundle`: A reference to the `Bundle` to be processed.
    ///
    /// # Returns
    /// - A `Date` indicating the estimated completion time for processing the bundle.
    #[cfg(feature = "node_proc")]
    fn dry_run_process(&self, at_time: Date, bundle: &mut Bundle) -> Date;

    /// Simulates transmitting a `Bundle` within a specified time window.
    ///
    /// This method performs a dry-run simulation to check if the bundle can be transmitted
    /// within the provided start and end times, without actually transmitting the data.
    ///
    /// # Parameters
    /// - `waiting_since`: The arrival time at the transmiter (allows to calculate a retention time)
    /// - `start`: The start time of the transmission window.
    /// - `end`: The end time of the transmission window.
    /// - `bundle`: A reference to the `Bundle` to be transmitted.
    ///
    /// # Returns
    /// - `true` if the bundle can be transmitted within the time window, `false` otherwise.
    #[cfg(feature = "node_tx")]
    fn dry_run_tx(&self, waiting_since: Date, start: Date, end: Date, bundle: &Bundle) -> bool;

    /// Simulates receiving a `Bundle` within a specified time window.
    ///
    /// This method performs a dry-run simulation to check if the bundle can be received
    /// within the provided start and end times, without actually receiving the data.
    ///
    /// # Parameters
    /// - `start`: The start time of the reception window.
    /// - `end`: The end time of the reception window.
    /// - `bundle`: A reference to the `Bundle` to be received.
    ///
    /// # Returns
    /// - `true` if the bundle can be received within the time window, `false` otherwise.
    #[cfg(feature = "node_rx")]
    fn dry_run_rx(&self, start: Date, end: Date, bundle: &Bundle) -> bool;

    /// Schedules the processing of a `Bundle` at a specified time.
    ///
    /// This method schedules the actual processing of a bundle at a specified time and returns
    /// the estimated completion time for the processing task.
    ///
    /// # Parameters
    /// - `at_time`: The time at which the processing should start.
    /// - `bundle`: A reference to the `Bundle` to be processed.
    ///
    /// # Returns
    /// - A `Date` indicating the completion time for the processing task.
    #[cfg(feature = "node_proc")]
    fn schedule_process(&self, at_time: Date, bundle: &mut Bundle) -> Date;

    /// Schedules the transmission of a `Bundle` within a specified time window.
    ///
    /// This method schedules the actual transmission of a bundle, checking if it can be
    /// transmitted within the provided time window. If successful, the bundle is transmitted.
    ///
    /// # Parameters
    /// - `waiting_since`: The arrival time at the transmiter (allows to calculate a retention time)
    /// - `start`: The start time of the transmission window.
    /// - `end`: The end time of the transmission window.
    /// - `bundle`: A reference to the `Bundle` to be transmitted.
    ///
    /// # Returns
    /// - `true` if the transmission is successfully scheduled within the window, `false` otherwise.
    #[cfg(feature = "node_tx")]
    fn schedule_tx(&mut self, waiting_since: Date, start: Date, end: Date, bundle: &Bundle)
        -> bool;

    /// Schedules the reception of a `Bundle` within a specified time window.
    ///
    /// This method schedules the actual reception of a bundle, checking if it can be received
    /// within the provided time window. If successful, the bundle is received.
    ///
    /// # Parameters
    /// - `start`: The start time of the reception window.
    /// - `end`: The end time of the reception window.
    /// - `bundle`: A reference to the `Bundle` to be received.
    ///
    /// # Returns
    /// - `true` if the reception is successfully scheduled within the window, `false` otherwise.
    #[cfg(feature = "node_rx")]
    fn schedule_rx(&mut self, start: Date, end: Date, bundle: &Bundle) -> bool;
}

/// Implementation of `NodeManager` for boxed types that implement `NodeManager`.
impl<NM: NodeManager> NodeManager for Box<NM> {
    /// Delegates the dry_run method to the boxed object.
    #[cfg(feature = "node_proc")]
    fn dry_run_process(&self, at_time: Date, bundle: &mut Bundle) -> Date {
        (**self).dry_run_process(at_time, bundle)
    }
    /// Delegates the dry_run method to the boxed object.
    #[cfg(feature = "node_tx")]
    fn dry_run_tx(&self, waiting_since: Date, start: Date, end: Date, bundle: &Bundle) -> bool {
        (**self).dry_run_tx(waiting_since, start, end, bundle)
    }
    /// Delegates the dry_run method to the boxed object.
    #[cfg(feature = "node_rx")]
    fn dry_run_rx(&self, start: Date, end: Date, bundle: &Bundle) -> bool {
        (**self).dry_run_rx(start, end, bundle)
    }
    /// Delegates the schedule method to the boxed object.
    #[cfg(feature = "node_proc")]
    fn schedule_process(&self, at_time: Date, bundle: &mut Bundle) -> Date {
        (**self).schedule_process(at_time, bundle)
    }
    /// Delegates the schedule method to the boxed object.
    #[cfg(feature = "node_tx")]
    fn schedule_tx(
        &mut self,
        waiting_since: Date,
        start: Date,
        end: Date,
        bundle: &Bundle,
    ) -> bool {
        (**self).dry_run_tx(waiting_since, start, end, bundle)
    }
    /// Delegates the schedule method to the boxed object.
    #[cfg(feature = "node_rx")]
    fn schedule_rx(&mut self, start: Date, end: Date, bundle: &Bundle) -> bool {
        (**self).dry_run_rx(start, end, bundle)
    }
}

/// Implementation of `NodeManager` for boxed dynamic types (`Box<dyn ContactManager>`).
impl NodeManager for Box<dyn NodeManager> {
    /// Delegates the dry_run method to the boxed object.
    #[cfg(feature = "node_proc")]
    fn dry_run_process(&self, at_time: Date, bundle: &mut Bundle) -> Date {
        (**self).dry_run_process(at_time, bundle)
    }
    /// Delegates the dry_run method to the boxed object.
    #[cfg(feature = "node_tx")]
    fn dry_run_tx(&self, waiting_since: Date, start: Date, end: Date, bundle: &Bundle) -> bool {
        (**self).dry_run_tx(waiting_since, start, end, bundle)
    }
    /// Delegates the dry_run method to the boxed object.
    #[cfg(feature = "node_rx")]
    fn dry_run_rx(&self, start: Date, end: Date, bundle: &Bundle) -> bool {
        (**self).dry_run_rx(start, end, bundle)
    }
    /// Delegates the schedule method to the boxed object.
    #[cfg(feature = "node_proc")]
    fn schedule_process(&self, at_time: Date, bundle: &mut Bundle) -> Date {
        (**self).schedule_process(at_time, bundle)
    }
    /// Delegates the schedule method to the boxed object.
    #[cfg(feature = "node_tx")]
    fn schedule_tx(
        &mut self,
        waiting_since: Date,
        start: Date,
        end: Date,
        bundle: &Bundle,
    ) -> bool {
        (**self).dry_run_tx(waiting_since, start, end, bundle)
    }
    /// Delegates the schedule method to the boxed object.
    #[cfg(feature = "node_rx")]
    fn schedule_rx(&mut self, start: Date, end: Date, bundle: &Bundle) -> bool {
        (**self).dry_run_rx(start, end, bundle)
    }
}

/// A trait that extends NodeManager with runtime type conversion capabilities.
/// This trait provides methods to convert a type-erased NodeManager into a type-erased Any,
/// which enables safe runtime downcasting to concrete types.
///
/// Use case: your manager must be modified with extern means (e.g. informations on transmissions queues)
/// and you need to downcast to a concrete type to call custom methods of your manager
trait AsAny: NodeManager {
    /// Converts this type to a type-erased `Any` reference.
    ///
    /// This method allows for runtime type checking and downcasting through the
    /// standard `Any` trait. The returned reference can be used with
    /// `downcast_ref` to safely convert back to a concrete type.
    ///
    /// # Returns
    ///
    /// A borrowed reference to `dyn Any` that can be used for downcasting.
    fn as_any(&self) -> &dyn Any;

    /// Converts this type to a type-erased mutable `Any` reference.
    ///
    /// Similar to `as_any`, but provides mutable access. This enables
    /// downcasting to a mutable reference of the concrete type.
    ///
    /// # Returns
    ///
    /// A mutable reference to `dyn Any` that can be used for downcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Blanket implementation of `AsAny` for any type that implements both
/// `NodeManager` and `Any`.
///
/// This implementation allows any concrete type implementing `NodeManager`
/// to be converted to a type-erased `Any` reference, enabling runtime
/// type checking and downcasting capabilities.
///
/// # Type Parameters
///
/// * `T`: The concrete type implementing both `NodeManager` and `Any`
impl<T: NodeManager + Any> AsAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
