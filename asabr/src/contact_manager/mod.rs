// #[cfg(feature = "first_depleted")]
extern crate alloc;
use alloc::boxed::Box;
use core::fmt::Debug;

#[cfg(feature = "first_depleted")]
use crate::types::Volume;
use crate::{
    bundle::Bundle,
    contact::ContactInfo,
    types::{Date, TimeInterval},
};

pub mod legacy;
pub mod lex;
pub mod segmentation;

/// Data structure representing the transmission (tx) start, end, and related timing information.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContactManagerTxData {
    /// Timespan necessary to send all the data.
    pub tx_window: TimeInterval,
    /// Expiration time.
    pub expiration: Date,
    /// Timespan during wich data are received.
    pub rx_window: TimeInterval,
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
        contact_lifespan: TimeInterval,
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
        contact_lifespan: TimeInterval,
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

    /// For ETO compatibility. Required with "manual_queueing" compilation feature.
    ///
    /// # Arguments
    ///
    /// * `bundle` - The bundle to be enqueued (it just checks its volume).
    ///
    /// # Returns
    ///
    /// true if manual enqueue is allowed, false otherwise
    #[cfg(feature = "manual_queueing")]
    fn manual_enqueue(&mut self, _bundle: &Bundle) -> bool {
        false
    }

    /// For ETO compatibility. Required with "manual_queueing" compilation feature.
    ///
    /// # Arguments
    ///
    /// * `bundle` - The bundle to be dequeued (it just checks its volume).
    ///
    /// # Returns
    ///
    /// true if manual dequeue is allowed, false otherwise
    #[cfg(feature = "manual_queueing")]
    fn manual_dequeue(&mut self, _bundle: &Bundle) -> bool {
        false
    }

    /// Finalize the initialization of the contact and notify if the initialization is consistent.
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

/// Implementation of `ContactManager` for dynamic types (eg `Box<dyn ContactManager>`).
impl<T: AsMut<dyn ContactManager> + AsRef<dyn ContactManager>> ContactManager for T {
    /// Delegates the dry run method to the boxed object.
    fn dry_run_tx(
        &self,
        contact_data: TimeInterval,
        at_time: Date,
        bundle: &Bundle,
    ) -> Option<ContactManagerTxData> {
        self.as_ref().dry_run_tx(contact_data, at_time, bundle)
    }
    /// Delegates the schedule method to the boxed object.
    fn schedule_tx(
        &mut self,
        contact_lifespan: TimeInterval,
        at_time: Date,
        bundle: &Bundle,
    ) -> Option<ContactManagerTxData> {
        self.as_mut().schedule_tx(contact_lifespan, at_time, bundle)
    }

    /// Delegates the try_init method to the boxed object.
    fn try_init(&mut self, contact_data: &ContactInfo) -> bool {
        self.as_mut().try_init(contact_data)
    }

    #[cfg(feature = "first_depleted")]
    /// Delegates the get_original_volume method to the boxed object.
    fn get_original_volume(&self) -> Volume {
        self.as_ref().get_original_volume()
    }
    /// Delegates the manual_enqueue method to the boxed object.
    #[cfg(feature = "manual_queueing")]
    fn manual_enqueue(&mut self, bundle: &Bundle) -> bool {
        self.as_mut().manual_enqueue(bundle)
    }
    /// Delegates the manual_dequeue method to the boxed object.
    #[cfg(feature = "manual_queueing")]
    fn manual_dequeue(&mut self, bundle: &Bundle) -> bool {
        self.as_mut().manual_dequeue(bundle)
    }
}

// Check that the above work, in particular, for Boxes
assert_impl_all! {Box<dyn ContactManager>: ContactManager}

/// This macro implement the ContactManager trait for you on a wrapper struct where the element 0 is the underlying
/// contact manager, by forwarding all calls to it
#[macro_export]
macro_rules! transparent_CM {
    ($T:ty) => {
        impl $crate::contact_manager::ContactManager for $T {
            fn dry_run_tx(
                &self,
                contact_data: $crate::types::TimeInterval,
                at_time: $crate::types::Date,
                bundle: &$crate::bundle::Bundle,
            ) -> Option<$crate::contact_manager::ContactManagerTxData> {
                self.0.dry_run_tx(contact_data, at_time, bundle)
            }

            fn schedule_tx(
                &mut self,
                contact_lifespan: $crate::types::TimeInterval,
                at_time: $crate::types::Date,
                bundle: &$crate::bundle::Bundle,
            ) -> Option<$crate::contact_manager::ContactManagerTxData> {
                self.0.schedule_tx(contact_lifespan, at_time, bundle)
            }

            fn try_init(&mut self, contact_data: &$crate::contact::ContactInfo) -> bool {
                self.0.try_init(contact_data)
            }

            #[cfg(feature = "first_depleted")]
            fn get_original_volume(&self) -> $crate::types::Volume {
                self.0.get_original_volume()
            }
            #[cfg(feature = "manual_queueing")]
            fn manual_enqueue(&mut self, bundle: &$crate::contact_manager::Bundle) -> bool {
                self.0.manual_enqueue(bundle)
            }
            #[cfg(feature = "manual_queueing")]
            fn manual_dequeue(&mut self, bundle: &$crate::contact_manager::Bundle) -> bool {
                self.0.manual_dequeue(bundle)
            }
        }
    };
}
