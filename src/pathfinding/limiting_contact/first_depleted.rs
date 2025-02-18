use crate::{
    contact::Contact, contact_manager::ContactManager, create_new_alternative_path_variant,
};

/// Compares the original transmission volume of two `Contact`s and determines if the first
/// contact (`a`) had less volume than the second contact (`b`) after initialization.
///
/// # Parameters
///
/// * `a` - A reference to the first `Contact` to compare.
/// * `b` - A reference to the second `Contact` to compare.
///
/// # Returns
///
/// A boolean value:
/// * `true` if `a` has a smaller original transmission volume than `b`.
/// * `false` otherwise.
fn had_less_volume_than<CM: ContactManager>(a: &Contact<CM>, b: &Contact<CM>) -> bool {
    return a.manager.get_original_volume() < b.manager.get_original_volume();
}

create_new_alternative_path_variant!(FirstDepleted, had_less_volume_than);
