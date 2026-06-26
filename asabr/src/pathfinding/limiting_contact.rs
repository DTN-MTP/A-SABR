#![cfg(feature = "contact_suppression")]
extern crate alloc;
use alloc::{boxed::Box, vec, vec::Vec};

use core::cmp::Ordering;
use core::marker::PhantomData;

use crate::bundle::Bundle;
use crate::contact::Contact;
use crate::contact_manager::ContactManager;
use crate::errors::ASABRError;
use crate::multigraph::{ContactRef, Multigraph, NodeRef, RNodeRef};
use crate::node_manager::NodeManager;
use crate::pathfinding::{PathFindingOutput, Pathfinding};
use crate::types::Date;

#[cfg(feature = "first_depleted")]

/// Comparison between two contacts using their original volume as criteria.
/// Intended for use with Suppressor (asabr::limiting_contact::Suppressor)
/// Have unspecified behavior on NaN
pub fn had_less_volume_than<CM: ContactManager>(a: &Contact<CM>, b: &Contact<CM>) -> Ordering {
    a.manager
        .get_original_volume()
        .partial_cmp(&b.manager.get_original_volume())
        .unwrap_or(Ordering::Equal)
        .reverse()
}

/// Comparison between two contacts using their expiration.
/// Intended for use with Suppressor (asabr::limiting_contact::Suppressor)
pub fn ends_earlier_than<CM: ContactManager>(a: &Contact<CM>, b: &Contact<CM>) -> Ordering {
    a.lifespan.end.cmp(&b.lifespan.end).reverse()
}

/// Retrieves the next `Contact` to suppress based on the provided suppression function.
///
/// This function navigates through the provided route stage to identify the `Contact` that
/// is best suited for suppression, according to the specified comparison function
/// (`better_for_suppression_than_fn`). It iterates through the route's contacts to determine
/// the one that should be suppressed next.
///
/// # Parameters
///
/// * `route` - A reference-counted, mutable `RouteStage` representing the current routing stage.
/// * `better_for_suppression_than_fn` - A function pointer used to compare two `Contact`s and
///   determine which is better for suppression.
///
/// # Returns
///
/// An `Option` containing a reference-counted, mutable `Contact` that should be suppressed, if one
/// is found; otherwise, `None`.
pub fn get_next_to_suppress<'id, 'a, NM: NodeManager, CM: ContactManager>(
    graph: &Multigraph<'id, NM, CM>,
    paths: &PathFindingOutput<'id, 'a>,
    destination: NodeRef<'id>,
    compare: fn(&Contact<CM>, &Contact<CM>) -> Ordering,
) -> Option<ContactRef<'id>> {
    let iter = paths.full_path_rev(destination, graph)?;

    let compare = |a: &ContactRef<'id>, b: &ContactRef<'id>| compare(&graph[a], &graph[b]);

    iter.filter_map(|frag| frag.via.map(|via| via.contact))
        .max_by(compare)
}

/// Pathfinding using the sub-pathfinder and contact suppression each time a route is searched.
/// The maximum contact for compare (Self::new(..) argument) being blacklisted for future search on the same destination
pub struct Suppressor<
    'id,
    P: Pathfinding<'id, NM, CM, NodeRef<'id>>,
    NM: NodeManager,
    CM: ContactManager,
> {
    pathfinder: P,
    function: fn(&Contact<CM>, &Contact<CM>) -> Ordering,
    suppressed: Box<[Vec<ContactRef<'id>>]>,
    _phantom: PhantomData<fn(NM, CM)>,
}

impl<'id, P: Pathfinding<'id, NM, CM, NodeRef<'id>>, NM: NodeManager, CM: ContactManager>
    Suppressor<'id, P, NM, CM>
{
    pub fn new(
        pathfinder: P,
        function: fn(&Contact<CM>, &Contact<CM>) -> Ordering,
        graph: &Multigraph<'id, NM, CM>,
    ) -> Self {
        Self {
            pathfinder,
            function,
            suppressed: vec![Vec::new(); graph.get_routable_count()].into_boxed_slice(),
            _phantom: PhantomData,
        }
    }
}
impl<'id, P: Pathfinding<'id, NM, CM, NodeRef<'id>>, NM: NodeManager, CM: ContactManager>
    Pathfinding<'id, NM, CM, NodeRef<'id>> for Suppressor<'id, P, NM, CM>
{
    /// Perform pathfinding with the underlying pathfinder, using a contact suppression map.
    /// Warning: can permanently (aka, modify graph state without full rollback) de-suppress contacts (but cannot suppress some)
    fn find_path<'a>(
        &'a mut self,
        multigraph: &mut Multigraph<'id, NM, CM>,
        current_time: Date,
        source: RNodeRef<'id>,
        bundle: &Bundle,
        destination: &mut NodeRef<'id>,
    ) -> Result<Option<PathFindingOutput<'id, 'a>>, ASABRError> {
        let idx = multigraph.into_usize(*destination);
        let suppressions = &mut self.suppressed[idx];
        for ct in suppressions.iter() {
            multigraph[ct].suppressed = true
        }

        let r = self
            .pathfinder
            .find_path(multigraph, current_time, source, bundle, destination);

        for ct in suppressions.iter() {
            multigraph[ct].suppressed = false
        }
        if let Ok(Some(path)) = &r {
            if let Some(to_suppress) =
                get_next_to_suppress(multigraph, &path, *destination, self.function)
            {
                suppressions.push(to_suppress);
            }
        }
        r
    }
}
