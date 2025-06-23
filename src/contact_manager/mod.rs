use std::any::Any;

use crate::{
    bundle::Bundle,
    contact::ContactInfo,
    types::{Date, Duration},
};

#[cfg(feature = "first_depleted")]
use crate::types::Volume;

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
