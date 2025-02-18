use crate::{
    contact::Contact, contact_manager::ContactManager, create_new_alternative_path_variant,
};

/// Compares the original contact end time of two `Contact`s and determines if the first
/// contact (`a`) terminates earlier than (`b`).
///
/// # Parameters
///
/// * `a` - A reference to the first `Contact` to compare.
/// * `b` - A reference to the second `Contact` to compare.
///
/// # Returns
///
/// A boolean value:
/// * `true` if `a` ends earlier than `b`.
/// * `false` otherwise.
fn ends_earlier_than<CM: ContactManager>(a: &Contact<CM>, b: &Contact<CM>) -> bool {
    return a.info.end < b.info.end;
}

create_new_alternative_path_variant!(FirstEnding, ends_earlier_than);
