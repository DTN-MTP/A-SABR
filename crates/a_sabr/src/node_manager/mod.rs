use crate::{bundle::Bundle, types::Date};

pub mod none;

macro_rules! define_node_manager {
    ($($bounds:tt)*) => {
        /// MISSING DOC COMMENT
        /// MISSING DOC COMMENT
        /// MISSING DOC COMMENT
        pub trait NodeRx: $($bounds)* {
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
            fn dry_run_rx(&self, start: Date, end: Date, bundle: &Bundle) -> bool;

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
            fn schedule_rx(&mut self, start: Date, end: Date, bundle: &Bundle) -> bool;
        }

        /// MISSING DOC COMMENT
        /// MISSING DOC COMMENT
        /// MISSING DOC COMMENT
        pub trait NodeTx: $($bounds)* {
            /// Simulates transmitting a `Bundle` within a specified time window.
            ///
            /// This method performs a dry-run simulation to check if the bundle can be transmitted
            /// within the provided start and end times, without actually transmitting the data.
            ///
            /// # Parameters
            /// - `waiting_since`: The arrival time at the transmitter (allows to calculate a retention time)
            /// - `start`: The start time of the transmission window.
            /// - `end`: The end time of the transmission window.
            /// - `bundle`: A reference to the `Bundle` to be transmitted.
            ///
            /// # Returns
            /// - `true` if the bundle can be transmitted within the time window, `false` otherwise.
            fn dry_run_tx(&self, waiting_since: Date, start: Date, end: Date, bundle: &Bundle) -> bool;

            /// Schedules the transmission of a `Bundle` within a specified time window.
            ///
            /// This method schedules the actual transmission of a bundle, checking if it can be
            /// transmitted within the provided time window. If successful, the bundle is transmitted.
            ///
            /// # Parameters
            /// - `waiting_since`: The arrival time at the transmitter (allows to calculate a retention time)
            /// - `start`: The start time of the transmission window.
            /// - `end`: The end time of the transmission window.
            /// - `bundle`: A reference to the `Bundle` to be transmitted.
            ///
            /// # Returns
            /// - `true` if the transmission is successfully scheduled within the window, `false` otherwise.
            fn schedule_tx(&mut self, waiting_since: Date, start: Date, end: Date, bundle: &Bundle) -> bool;
        }

        /// A trait for managing and scheduling operations on nodes in a network.
        ///
        /// The `NodeManager` trait defines methods for dry-run (simulation) and actual scheduling
        /// of processing, transmission (tx), and reception (rx) of a `Bundle` at specified times.
        /// This trait is useful for implementing custom logic for nodes that need to manage bundle
        /// processing and data transfer in a time-dependent manner.
        pub trait NodeManager: NodeRx + NodeTx + $($bounds)* {
            /// Simulates processing a `Bundle` at a specified time.
            ///
            /// This method performs a dry run to estimate the processing time of a bundle without
            /// actually executing the process. It returns the estimated completion time.
            ///
            /// # Parameters
            /// - `at_time`: The time at which the dry-run process simulation should start.
            /// - `bundle`: A mutable reference to the `Bundle` to be processed.
            ///
            /// # Returns
            /// - A `Date` indicating the estimated completion time for processing the bundle.
            #[cfg(feature = "node_proc")]
            fn dry_run_process(&self, at_time: Date, bundle: &mut Bundle) -> Date;

            /// Schedules the processing of a `Bundle` at a specified time.
            ///
            /// This method schedules the actual processing of a bundle at a specified time and returns
            /// the estimated completion time for the processing task.
            ///
            /// # Parameters
            /// - `at_time`: The time at which the processing should start.
            /// - `bundle`: A mutable reference to the `Bundle` to be processed.
            ///
            /// # Returns
            /// - A `Date` indicating the completion time for the processing task.
            #[cfg(feature = "node_proc")]
            fn schedule_process(&self, at_time: Date, bundle: &mut Bundle) -> Date;
        }

        /// Implementation of `NodeRx` for boxed types that implement `NodeRx`.
        impl<NR: NodeRx> NodeRx for Box<NR> {
            /// Delegates the dry_run method to the boxed object.
            fn dry_run_rx(&self, start: Date, end: Date, bundle: &Bundle) -> bool {
                (**self).dry_run_rx(start, end, bundle)
            }
            /// Delegates the schedule method to the boxed object.
            fn schedule_rx(&mut self, start: Date, end: Date, bundle: &Bundle) -> bool {
                (**self).dry_run_rx(start, end, bundle)
            }
        }

        /// Implementation of `NodeTx` for boxed types that implement `NodeTx`.
        impl<NT: NodeTx> NodeTx for Box<NT> {
            /// Delegates the dry_run method to the boxed object.
            fn dry_run_tx(&self, waiting_since: Date, start: Date, end: Date, bundle: &Bundle) -> bool {
                (**self).dry_run_tx(waiting_since, start, end, bundle)
            }
            /// Delegates the schedule method to the boxed object.
            fn schedule_tx(
                &mut self,
                waiting_since: Date,
                start: Date,
                end: Date,
                bundle: &Bundle,
            ) -> bool {
                (**self).dry_run_tx(waiting_since, start, end, bundle)
            }
        }

        /// Implementation of `NodeManager` for boxed types that implement `NodeManager`.
        impl<NM: NodeManager> NodeManager for Box<NM> {
            /// Delegates the dry_run method to the boxed object.
            #[cfg(feature = "node_proc")]
            fn dry_run_process(&self, at_time: Date, bundle: &mut Bundle) -> Date {
                (**self).dry_run_process(at_time, bundle)
            }
            /// Delegates the schedule method to the boxed object.
            #[cfg(feature = "node_proc")]
            fn schedule_process(&self, at_time: Date, bundle: &mut Bundle) -> Date {
                (**self).schedule_process(at_time, bundle)
            }
        }

        /// Implementation of `NodeRx` for boxed dynamic types (`Box<dyn NodeRx>`).
        impl NodeRx for Box<dyn NodeRx> {
            /// Delegates the dry_run method to the boxed object.
            fn dry_run_rx(&self, start: Date, end: Date, bundle: &Bundle) -> bool {
                (**self).dry_run_rx(start, end, bundle)
            }
            /// Delegates the schedule method to the boxed object.
            fn schedule_rx(&mut self, start: Date, end: Date, bundle: &Bundle) -> bool {
                (**self).dry_run_rx(start, end, bundle)
            }
        }

        /// Implementation of `NodeTx` for boxed dynamic types (`Box<dyn NodeTx>`).
        impl NodeTx for Box<dyn NodeTx> {
            /// Delegates the dry_run method to the boxed object.
            fn dry_run_tx(&self, waiting_since: Date, start: Date, end: Date, bundle: &Bundle) -> bool {
                (**self).dry_run_tx(waiting_since, start, end, bundle)
            }
            /// Delegates the schedule method to the boxed object.
            fn schedule_tx(
                &mut self,
                waiting_since: Date,
                start: Date,
                end: Date,
                bundle: &Bundle,
            ) -> bool {
                (**self).dry_run_tx(waiting_since, start, end, bundle)
            }
        }

        /// Implementation of `NodeManager` for boxed dynamic types (`Box<dyn NodeManager>`).
        impl NodeManager for Box<dyn NodeManager> {
            /// Delegates the dry_run method to the boxed object.
            #[cfg(feature = "node_proc")]
            fn dry_run_process(&self, at_time: Date, bundle: &mut Bundle) -> Date {
                (**self).dry_run_process(at_time, bundle)
            }
            /// Delegates the schedule method to the boxed object.
            #[cfg(feature = "node_proc")]
            fn schedule_process(&self, at_time: Date, bundle: &mut Bundle) -> Date {
                (**self).schedule_process(at_time, bundle)
            }
        }
    }
}

#[cfg(feature = "debug")]
define_node_manager!(std::fmt::Debug);

#[cfg(not(feature = "debug"))]
define_node_manager!();
