use crate::{
    contact::Contact, contact_manager::ContactManager, create_new_alternative_path_variant,
    node_manager::NodeManager,
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
fn ends_earlier_than<NM: NodeManager, CM: ContactManager>(
    a: &Contact<NM, CM>,
    b: &Contact<NM, CM>,
) -> bool {
    return a.info.end < b.info.end;
}

create_new_alternative_path_variant!(FirstEnding, ends_earlier_than);
