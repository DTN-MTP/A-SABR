#[cfg(feature = "first_depleted")]
use crate::types::Volume;
use crate::{
    bundle::Bundle,
    contact::ContactInfo,
    types::{Date, Duration},
};

pub mod legacy;
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
    /// Delegates the manual_enqueue method to the boxed object.
    #[cfg(feature = "manual_queueing")]
    fn manual_enqueue(&mut self, _bundle: &Bundle) -> bool {
        (**self).manual_enqueue(_bundle)
    }
    /// Delegates the manual_dequeue method to the boxed object.
    #[cfg(feature = "manual_queueing")]
    fn manual_dequeue(&mut self, _bundle: &Bundle) -> bool {
        (**self).manual_dequeue(_bundle)
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

    /// Delegates the try_init method to the boxed object.
    fn try_init(&mut self, contact_data: &ContactInfo) -> bool {
        (**self).try_init(contact_data)
    }

    #[cfg(feature = "first_depleted")]
    /// Delegates the get_original_volume method to the boxed object.
    fn get_original_volume(&self) -> Volume {
        (**self).get_original_volume()
    }
    /// Delegates the manual_enqueue method to the boxed object.
    #[cfg(feature = "manual_queueing")]
    fn manual_enqueue(&mut self, _bundle: &Bundle) -> bool {
        (**self).manual_enqueue(_bundle)
    }
    /// Delegates the manual_dequeue method to the boxed object.
    #[cfg(feature = "manual_queueing")]
    fn manual_dequeue(&mut self, _bundle: &Bundle) -> bool {
        (**self).manual_dequeue(_bundle)
    }
}
