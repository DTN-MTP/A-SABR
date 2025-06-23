use std::any::Any;

use crate::{
    bundle::Bundle,
    contact::ContactInfo,
    types::{Date, Duration},
};

#[cfg(feature = "first_depleted")]
use crate::types::Volume;

pub mod eto;
pub mod evl;
pub mod peto;
pub mod pevl;
pub mod pqd;
pub mod qd;
pub mod seg;

/// Data structure representing the transmission (tx) start, end, and related timing information.
pub struct ContactManagerTxData {
    /// The start time of the transmission.
    pub tx_start: Date,
    /// The end time of the transmission.
    pub tx_end: Date,
    /// The last bit transmission delay.
    pub delay: Duration,
    /// Expiration time.
    pub expiration: Date,
    /// The last bit arrival time (tx_end + delay).
    pub arrival: Date,
}

/// Trait for managing contact resources and scheduling data transmissions.
pub trait ContactManager {
    /// Simulate the transmission of a bundle to a contact at a given time.
    ///
    /// # Arguments
    ///
    /// * `contact_data` - Reference to the contact information.
    /// * `at_time` - The current time for scheduling purposes.
    /// * `bundle` - The data bundle to be transmitted.
    ///
    /// # Returns
    ///
    /// Optionally returns the `ContactManagerTxData` if the dry run is successful.
    fn dry_run_tx(
        &self,
        contact_data: &ContactInfo,
        at_time: Date,
        bundle: &Bundle,
    ) -> Option<ContactManagerTxData>;

    /// Schedule the transmission of a bundle based on the contact data and available free intervals.
    ///
    /// This method shall be called after a dry run ! Implementations might not ensure a clean behavior otherwise.
    ///
    /// # Arguments
    ///
    /// * `contact_data` - Reference to the contact information (unused in this implementation).
    /// * `at_time` - The current time for scheduling purposes.
    /// * `bundle` - The bundle to be transmitted.
    ///
    /// # Returns
    ///
    /// Optionally returns `ContactManagerTxData` if the bundle can be transmitted.
    fn schedule_tx(
        &mut self,
        contact_data: &ContactInfo,
        at_time: Date,
        bundle: &Bundle,
    ) -> Option<ContactManagerTxData>;

    /// For first depleted compatibility. Required with "first_depleted" compilation feature.
    ///
    /// # Returns
    ///
    /// Returns the maximum volume the contact had at initialization.
    #[cfg(feature = "first_depleted")]
    fn get_original_volume(&self) -> Volume;

    /// Finalize the initialize of the contact and notify if the initailization is consistent.
    ///
    /// # Arguments
    ///
    /// * `contact_data` - Reference to the contact information.
    ///
    /// # Returns
    ///
    /// Returns `true` if the initialization is consistent.
    fn try_init(&mut self, contact_data: &ContactInfo) -> bool;
}

/// Implementation of `ContactManager` for boxed types that implement `ContactManager`.
impl<CM: ContactManager> ContactManager for Box<CM> {
    /// Delegates the dry run method to the boxed object.
    fn dry_run_tx(
        &self,
        contact_data: &ContactInfo,
        at_time: Date,
        bundle: &Bundle,
    ) -> Option<ContactManagerTxData> {
        (**self).dry_run_tx(contact_data, at_time, bundle)
    }

    /// Delegates the schedule method to the boxed object.
    fn schedule_tx(
        &mut self,
        contact_data: &ContactInfo,
        at_time: Date,
        bundle: &Bundle,
    ) -> Option<ContactManagerTxData> {
        (**self).schedule_tx(contact_data, at_time, bundle)
    }
    /// Delegates the get_original_volume method to the boxed object.
    #[cfg(feature = "first_depleted")]
    fn get_original_volume(&self) -> Volume {
        (**self).get_original_volume()
    }

    /// Delegates the try_init method to the boxed object.
    fn try_init(&mut self, contact_data: &ContactInfo) -> bool {
        (**self).try_init(contact_data)
    }
}

/// Implementation of `ContactManager` for boxed dynamic types (`Box<dyn ContactManager>`).
impl ContactManager for Box<dyn ContactManager> {
    /// Delegates the dry run method to the boxed object.
    fn dry_run_tx(
        &self,
        contact_data: &ContactInfo,
        at_time: Date,
        bundle: &Bundle,
    ) -> Option<ContactManagerTxData> {
        (**self).dry_run_tx(contact_data, at_time, bundle)
    }
    /// Delegates the schedule method to the boxed object.
    fn schedule_tx(
        &mut self,
        contact_data: &ContactInfo,
        at_time: Date,
        bundle: &Bundle,
    ) -> Option<ContactManagerTxData> {
        (**self).schedule_tx(contact_data, at_time, bundle)
    }

    #[cfg(feature = "first_depleted")]
    /// Delegates the get_original_volume method to the boxed object.
    fn get_original_volume(&self) -> Volume {
        (**self).get_original_volume()
    }

    /// Delegates the try_init method to the boxed object.
    fn try_init(&mut self, contact_data: &ContactInfo) -> bool {
        (**self).try_init(contact_data)
    }
}

/// A trait that extends ContactManager with runtime type conversion capabilities.
/// This trait provides methods to convert a type-erased ContactManager into a type-erased Any,
/// which enables safe runtime downcasting to concrete types.
///
/// Use case: the manager must be modified with extern means (e.g. informations on transmissions queues)
/// and this needs to downcast the manager to a concrete type to call custom methods of the manager.
trait AsAny: ContactManager {
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
/// `ContactManager` and `Any`.
///
/// This implementation allows any concrete type implementing `ContactManager`
/// to be converted to a type-erased `Any` reference, enabling runtime
/// type checking and downcasting capabilities.
///
/// # Type Parameters
///
/// * `CM`: The concrete type implementing both `ContactManager` and `Any`
impl<CM: ContactManager + Any> AsAny for CM {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// A macro to conditionally implement `enqueue` and `dequeue` methods.
///
/// This macro takes a boolean value and implements the methods based on that value:
/// - When `true`, no methods are implemented.
/// - When `false`, the following methods are implemented:
///   - `enqueue`: Adds a bundle to the queue.
///   - `dequeue`: Removes a bundle from the queue.
#[macro_export]
macro_rules! impl_struct_conditional_methods {
    (true) => {
        // No enqueue or dequeue methods when $auto_update is true
    };

    (false) => {
        pub fn enqueue(&mut self, bundle: &crate::bundle::Bundle) {
            let new_size = self.queue_size + bundle.size;
            if new_size > self.original_volume {
                panic!("Queue will overflow the contact's volume");
            }
            self.queue_size = new_size;
        }

        pub fn dequeue(&mut self, bundle: &crate::bundle::Bundle) {
            if self.queue_size < bundle.size {
                panic!("Attempting to dequeue a bundle larger than the current queue size");
            }
            self.queue_size -= bundle.size;
        }
    };
}

/// A macro to generate a basic volume manager struct and its associated methods.
///
/// This macro creates a new struct that manages volume based on specified parameters:
/// - `manager_name`: The name of the generated volume manager struct.
/// - `add_delay`: A boolean indicating whether to add delay when scheduling.
/// - `auto_update`: A boolean indicating whether to automatically update the queue size.
///
/// See the documentation of the resulting implementations for more information.
#[macro_export]
macro_rules! generate_basic_volume_manager {

    ($manager_name:ident, $add_delay:tt, $auto_update:tt) => {
        /// A simple manager for handling volume and/or transmission delays (macro generated).
        ///
        #[doc = concat!(
            "`", stringify!($manager_name),"` compilation rules:\n",
            " * Consider the delay to offset the earliest transmission opportunity: `", stringify!($add_delay), "`.\n",
            " * Update automatically the booked volume (i.e. queue) upon schedule: `", stringify!($auto_update), "`."
        )]
        #[cfg_attr(feature = "debug", derive(Debug))]
        pub struct $manager_name {
            /// The data transmission rate.
            rate: crate::types::DataRate,
            /// The delay between transmissions.
            delay: crate::types::Duration,
            /// The volume scheduled for this contact.
            queue_size: crate::types::Volume,
            /// The total volume at initialization.
            original_volume: crate::types::Volume,
        }

        impl $manager_name {
            #[doc = concat!( "Creates a new `", stringify!($manager_name),"`  with specified average rate and delay.")]
            ///
            /// # Arguments
            ///
            /// * `rate` - The average data rate for this contact.
            /// * `delay` - The link delay for this contact.
            ///
            /// # Returns
            ///
             #[doc = concat!( " A new instance of  `", stringify!($manager_name),"`.")]
            pub fn new(rate: crate::types::DataRate, delay: crate::types::Duration) -> Self {
                Self {
                    rate,
                    delay,
                    queue_size: 0.0,
                    original_volume: 0.0,
                }
            }
            // Conditionally implement enqueue and dequeue only when $auto_update is false
            crate::impl_struct_conditional_methods!($auto_update);
        }
        impl crate::contact_manager::ContactManager for $manager_name {
            /// Simulates the transmission of a bundle based on the contact data and available free intervals.
            ///
            #[doc = concat!( "The transmission time start time will be offset by the queue size: ", stringify!($add_delay),"`.")]
            ///
            /// # Arguments
            ///
            /// * `contact_data` - Reference to the contact information (unused in this implementation).
            /// * `at_time` - The current time for scheduling purposes.
            /// * `bundle` - The bundle to be transmitted.
            ///
            /// # Returns
            ///
            /// Optionally returns `ContactManagerTxData` with transmission start and end times, or `None` if the bundle can't be transmitted.
            fn dry_run_tx(
                &self,
                contact_data: &crate::contact::ContactInfo,
                at_time: crate::types::Date,
                bundle: &crate::bundle::Bundle,
            ) -> Option<crate::contact_manager::ContactManagerTxData> {
                if bundle.size > self.original_volume - self.queue_size {
                    return None;
                }
                let mut tx_start = if contact_data.start > at_time {
                    contact_data.start
                } else {
                    at_time
                };

                // Conditionally add queue delay based on $add_delay
                if $add_delay {
                    tx_start += self.queue_size / self.rate;
                }

                let tx_end = tx_start + bundle.size / self.rate;
                if tx_end > contact_data.end {
                    return None;
                }
                Some(crate::contact_manager::ContactManagerTxData {
                    tx_start,
                    tx_end,
                    delay: self.delay,
                    expiration: contact_data.end,
                    arrival: self.delay + tx_end,
                })
            }

            /// Schedule the transmission of a bundle based on the contact data and available free intervals.
            ///
            /// This method shall be called after a dry run !Implementations might not ensure a clean behavior otherwise.
            #[doc = concat!( "The queue volume will be updated by this method: ", stringify!($auto_update),"`.")]
            ///
            /// # Arguments
            ///
            /// * `contact_data` - Reference to the contact information (unused in this implementation).
            /// * `at_time` - The current time for scheduling purposes.
            /// * `bundle` - The bundle to be transmitted.
            ///
            /// # Returns
            ///
            /// Optionally returns `ContactManagerTxData` with transmission start and end times, or `None` if the bundle can't be transmitted.
            fn schedule_tx(
                &mut self,
                contact_data: &crate::contact_manager::ContactInfo,
                at_time: crate::types::Date,
                bundle: &crate::bundle::Bundle,
            ) -> Option<crate::contact_manager::ContactManagerTxData> {
                if let Some(data) = self.dry_run_tx(contact_data, at_time, bundle) {
                    // Conditionally update queue size based on $auto_update
                    if $auto_update {
                        self.queue_size += bundle.size;
                    }
                    return Some(data);
                }
                None
            }

            /// Initializes the segmentation manager by checking that rate and delay intervals have no gaps.
            ///
            /// # Arguments
            ///
            /// * `contact_data` - Reference to the contact information.
            ///
            /// # Returns
            ///
            /// Returns `true` if initialization is successful, or `false` if there are gaps in the intervals.
            fn try_init(&mut self, contact_data: &crate::contact::ContactInfo) -> bool {
                self.original_volume = (contact_data.end - contact_data.start) * self.rate;
                true
            }

            /// Returns the original volume of the object.
            ///
            /// # Returns
            ///
            /// A `Volume` representing the original volume.
            #[cfg(feature = "first_depleted")]
            fn get_original_volume(&self) -> crate::types::Volume {
                self.original_volume
            }
        }

        /// Implements the DispatchParser to allow dynamic parsing.
        impl crate::parsing::DispatchParser<$manager_name> for $manager_name {}

        #[doc = concat!("Implements the `Parser` trait for ", stringify!($manager_name),"`, allowing the manager to be parsed from a lexer.")]
        /// Implements the `Parser` trait for `SegmentationManager`, allowing the manager to be parsed from a lexer.
        impl crate::parsing::Parser<$manager_name> for $manager_name {
            #[doc = concat!("Parses a `", stringify!($manager_name),"` from the lexer, extracting the rate and delay intervals.")]
            ///
            /// # Arguments
            ///
            /// * `lexer` - The lexer used for parsing tokens.
            /// * `_sub` - An optional map for handling custom parsing logic (unused here).
            ///
            /// # Returns
            ///
            /// Returns a `ParsingState` indicating whether parsing was successful (`Finished`) or encountered an error (`Error`).
            fn parse(
                lexer: &mut dyn crate::parsing::Lexer,
            ) -> crate::parsing::ParsingState<$manager_name> {
                let delay: crate::types::Duration;
                let rate: crate::types::DataRate;

                let rate_state = <crate::types::DataRate as crate::types::Token<crate::types::DataRate>>::parse(lexer);
                match rate_state {
                    crate::parsing::ParsingState::Finished(value) => rate = value,
                    crate::parsing::ParsingState::Error(msg) => return crate::parsing::ParsingState::Error(msg),
                    crate::parsing::ParsingState::EOF => {
                        return crate::parsing::ParsingState::Error(format!(
                            "Parsing failed ({})",
                            lexer.get_current_position()
                        ))
                    }
                }

                let delay_state = <crate::types::Duration as crate::types::Token<crate::types::Duration>>::parse(lexer);
                match delay_state {
                    crate::parsing::ParsingState::Finished(value) => delay = value,
                    crate::parsing::ParsingState::Error(msg) => return crate::parsing::ParsingState::Error(msg),
                    crate::parsing::ParsingState::EOF => {
                        return crate::parsing::ParsingState::Error(format!(
                            "Parsing failed ({})",
                            lexer.get_current_position()
                        ))
                    }
                }

                crate::parsing::ParsingState::Finished($manager_name::new(rate, delay))
            }
        }
    }
}

#[cfg(feature = "enable_priority")]
/// A macro to conditionally implement `enqueue` and `dequeue` methods.
///
/// This macro takes a boolean value and implements the methods based on that value:
/// - When `true`, no methods are implemented.
/// - When `false`, the following methods are implemented:
///   - `enqueue`: Adds a bundle to the queue.
///   - `dequeue`: Removes a bundle from the queue.
#[macro_export]
macro_rules! impl_struct_conditional_methods_with_priority {
    (true) => {
        // No enqueue or dequeue methods when $auto_update is true
    };

    (false) => {
        pub fn enqueue(&mut self, bundle: &crate::bundle::Bundle) {
            self.queue_size[bundle.priority as usize] += bundle.size;
        }

        pub fn dequeue(&mut self, bundle: &crate::bundle::Bundle) {
            self.queue_size[bundle.priority as usize] -= bundle.size;
        }
    };
}

#[cfg(feature = "enable_priority")]
/// A macro to generate a basic volume manager struct with priority support and its associated methods.
///
/// This macro creates a new struct that manages volume based on specified parameters:
/// - `manager_name`: The name of the generated volume manager struct.
/// - `add_delay`: A boolean indicating whether to add delay when scheduling.
/// - `auto_update`: A boolean indicating whether to automatically update the queue size.
///
/// See the documentation of the resulting implementations for more information.
#[macro_export]
macro_rules! generate_basic_volume_manager_with_priority {
    ($manager_name:ident, $add_delay:tt, $auto_update:tt) => {
        /// A simple manager for handling volume and/or transmission delays (macro generated).
        ///
        #[doc = concat!(
            "`", stringify!($manager_name),"` compilation rules:\n",
            " * Consider the delay to offset the earliest transmission opportunity: `", stringify!($add_delay), "`.\n",
            " * Update automatically the booked volume (i.e. queue) upon schedule: `", stringify!($auto_update), "`."
        )]
        #[cfg_attr(feature = "debug", derive(Debug))]
        pub struct $manager_name {
            /// The data transmission rate.
            rate: crate::types::DataRate,
            /// The delay between transmissions.
            pub delay: crate::types::Duration,
            /// The volume scheduled for this contact.
            pub queue_size: [crate::types::Volume;3],
            /// The total volume at initialization.
            original_volume: crate::types::Volume,
            /// The current maximum available volume in 3 different priorities for this contact.
            pub mav: [crate::types::Volume; 3],
        }

        impl $manager_name {
            #[doc = concat!( "Creates a new `", stringify!($manager_name),"`  with specified average rate and delay.")]
            ///
            /// # Arguments
            ///
            /// * `rate` - The average data rate for this contact.
            /// * `delay` - The link delay for this contact.
            ///
            /// # Returns
            ///
             #[doc = concat!( " A new instance of  `", stringify!($manager_name),"`.")]
            pub fn new(rate: crate::types::DataRate, delay: crate::types::Duration, original_mav: [crate::types::Volume; 3]) -> Self {
                Self {
                    rate,
                    delay,
                    queue_size: [0.0; 3],
                    original_volume: 0.0,
                    mav: original_mav,
                }
            }

            /// Get Maximum Available Volume or queue size for a given priority.
            #[inline(always)]
            pub fn get_vol(
                &self,
                vols: &[crate::types::Volume; 3],
                priority: crate::types::Priority,
            ) -> crate::types::Volume {
                *vols.get(priority as usize).unwrap_or(&0.0) // Return 0 if priority is out of range / not defined.
            }

            /// Update Maximum Available Volume after scheduling a bundle.
            #[inline(always)]
            fn update_mav(&mut self, vol: crate::types::Volume, priority: crate::types::Priority) {
                let p = priority as usize;
                if p < self.mav.len() {
                    // Deduct volume from the specified and lower prioritys' MAV.
                    for i in (0..p).rev() {
                        if self.mav[i] > vol {
                            self.mav[i] -= vol;
                        } else {
                            for j in 0..i {
                                self.mav[j] = 0.0; // Set all lower priorities to 0.
                            }
                            break;
                        }
                    }
                }
            }

            // Conditionally implement enqueue and dequeue only when $auto_update is false
            crate::impl_struct_conditional_methods_with_priority!($auto_update);
        }
        impl crate::contact_manager::ContactManager for $manager_name {
            /// Simulates the transmission of a bundle based on the contact data and available free intervals.
            ///
            #[doc = concat!( "The transmission time start time will be offset by the queue size: ", stringify!($add_delay),"`.")]
            ///
            /// # Arguments
            ///
            /// * `contact_data` - Reference to the contact information (unused in this implementation).
            /// * `at_time` - The current time for scheduling purposes.
            /// * `bundle` - The bundle to be transmitted.
            ///
            /// # Returns
            ///
            /// Optionally returns `ContactManagerTxData` with transmission start and end times, or `None` if the bundle can't be transmitted.
            fn dry_run_tx(
                &self,
                contact_data: &crate::contact::ContactInfo,
                at_time: crate::types::Date,
                bundle: &crate::bundle::Bundle,

            ) -> Option<crate::contact_manager::ContactManagerTxData> {
                let mut tx_start = if contact_data.start > at_time { // EVL
                    contact_data.start
                } else {
                    at_time
                };

                // Conditionally add queue delay based on $add_delay
                if $add_delay {
                    let mut total_queue_size = 0.0;
                    for i in (bundle.priority as usize)..self.queue_size.len() {
                        total_queue_size += self.queue_size[i];
                    }

                    if !$auto_update { // ETO
                        tx_start += total_queue_size / self.rate;
                    } else {  // QD
                        if contact_data.start > at_time {
                            tx_start = contact_data.start + total_queue_size / self.rate
                        } else {
                            tx_start = contact_data.start
                        }
                    }
                }

                let tx_end = tx_start + bundle.size / self.rate;
                if tx_end > contact_data.end {
                    return None;
                }

                let arrival = self.delay + tx_end;
                if arrival > bundle.expiration {
                    return None;
                }

                let max_volume = (tx_end - tx_start) * self.rate;
                if bundle.size > max_volume.min(self.get_vol(&self.mav, bundle.priority)) {
                    return None;
                }

                Some(crate::contact_manager::ContactManagerTxData {
                    tx_start,
                    tx_end,
                    delay: self.delay,
                    expiration: contact_data.end,
                    arrival,
                })
            }

            /// Schedule the transmission of a bundle based on the contact data and available free intervals.
            ///
            /// This method shall be called after a dry run !Implementations might not ensure a clean behavior otherwise.
            #[doc = concat!( "The queue volume will be updated by this method: ", stringify!($auto_update),"`.")]
            ///
            /// # Arguments
            ///
            /// * `contact_data` - Reference to the contact information (unused in this implementation).
            /// * `at_time` - The current time for scheduling purposes.
            /// * `bundle` - The bundle to be transmitted.
            ///
            /// # Returns
            ///
            /// Optionally returns `ContactManagerTxData` with transmission start and end times, or `None` if the bundle can't be transmitted.
            fn schedule_tx(
                &mut self,
                contact_data: &crate::contact_manager::ContactInfo,
                at_time: crate::types::Date,
                bundle: &crate::bundle::Bundle,
            ) -> Option<crate::contact_manager::ContactManagerTxData> {
                if let Some(data) = self.dry_run_tx(contact_data, at_time, bundle) {
                    self.update_mav(bundle.size, bundle.priority);
                    // Conditionally update queue size based on $auto_update
                    // Can overflow with overbooking
                    if $auto_update {
                        self.queue_size[bundle.priority as usize] += bundle.size;
                    }
                    return Some(data);
                }
                None
            }

            /// Initializes the segmentation manager by checking that rate and delay intervals have no gaps.
            ///
            /// # Arguments
            ///
            /// * `contact_data` - Reference to the contact information.
            ///
            /// # Returns
            ///
            /// Returns `true` if initialization is successful, or `false` if there are gaps in the intervals.
            fn try_init(&mut self, contact_data: &crate::contact::ContactInfo) -> bool {
                self.original_volume = (contact_data.end - contact_data.start) * self.rate;
                true
            }

            /// Returns the original volume of the object.
            ///
            /// # Returns
            ///
            /// A `Volume` representing the original volume.
            #[cfg(feature = "first_depleted")]
            fn get_original_volume(&self) -> crate::types::Volume {
                self.original_volume
            }
        }

        /// Implements the DispatchParser to allow dynamic parsing.
        impl crate::parsing::DispatchParser<$manager_name> for $manager_name {}

        #[doc = concat!("Implements the `Parser` trait for ", stringify!($manager_name),"`, allowing the manager to be parsed from a lexer.")]
        /// Implements the `Parser` trait for `SegmentationManager`, allowing the manager to be parsed from a lexer.
        impl crate::parsing::Parser<$manager_name> for $manager_name {
            #[doc = concat!("Parses a `", stringify!($manager_name),"` from the lexer, extracting the rate and delay intervals.")]
            ///
            /// # Arguments
            ///
            /// * `lexer` - The lexer used for parsing tokens.
            /// * `_sub` - An optional map for handling custom parsing logic (unused here).
            ///
            /// # Returns
            ///
            /// Returns a `ParsingState` indicating whether parsing was successful (`Finished`) or encountered an error (`Error`).
            fn parse(
                lexer: &mut dyn crate::parsing::Lexer,
            ) -> crate::parsing::ParsingState<$manager_name> {
                let delay: crate::types::Duration;
                let rate: crate::types::DataRate;

                let rate_state = <crate::types::DataRate as crate::types::Token<crate::types::DataRate>>::parse(lexer);
                match rate_state {
                    crate::parsing::ParsingState::Finished(value) => rate = value,
                    crate::parsing::ParsingState::Error(msg) => return crate::parsing::ParsingState::Error(msg),
                    crate::parsing::ParsingState::EOF => {
                        return crate::parsing::ParsingState::Error(format!(
                            "Parsing failed ({})",
                            lexer.get_current_position()
                        ))
                    }
                }

                let delay_state = <crate::types::Duration as crate::types::Token<crate::types::Duration>>::parse(lexer);
                match delay_state {
                    crate::parsing::ParsingState::Finished(value) => delay = value,
                    crate::parsing::ParsingState::Error(msg) => return crate::parsing::ParsingState::Error(msg),
                    crate::parsing::ParsingState::EOF => {
                        return crate::parsing::ParsingState::Error(format!(
                            "Parsing failed ({})",
                            lexer.get_current_position()
                        ))
                    }
                }

                let mut original_mav = [0.0_f64; 3];
                for i in 0..3 {
                    match <crate::types::Volume as crate::types::Token<crate::types::Volume>>::parse(lexer) {
                        crate::parsing::ParsingState::Finished(value) => original_mav[i] = value,
                        crate::parsing::ParsingState::Error(msg) => {
                            return crate::parsing::ParsingState::Error(msg)
                        }
                        crate::parsing::ParsingState::EOF => {
                            return crate::parsing::ParsingState::Error(format!(
                                "Parsing MAV of priority {} failed ({})",
                                i + 1,
                                lexer.get_current_position()
                            ))
                        }
                    }
                }
                crate::parsing::ParsingState::Finished($manager_name::new(rate, delay, original_mav))
            }
        }
    }
}
