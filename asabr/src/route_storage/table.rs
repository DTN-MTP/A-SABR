extern crate alloc;
use alloc::collections::BTreeMap;

use core::marker::PhantomData;

use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    errors::ASABRError,
    multigraph::Multigraph,
    node_manager::NodeManager,
    pathfinding::{PathFindingOutput, destination::FindableDest},
    types::Priority,
};

use super::PathsStorage;
/// A routing table that stores the routes for each destination/priority pair.
///
/// `RoutingTable` stores and selects the best available routes for bundles. The table allows
/// the storage of new routes and the selection of optimal routes based on the `Distance<NM, CM>` trait.
///
/// # Type Parameters
/// - `NM`: graph `NodeManager`
/// - `CM`: graph `ContactManager`, handling contacts within the network.
/// - `D`: Destination key type used to index stored routes.
#[derive(Debug, Default)]
pub struct RoutingTable<'id, D: FindableDest<'id, NM, CM>, NM: NodeManager, CM: ContactManager> {
    cache: BTreeMap<(usize, Priority), PathFindingOutput<'id, 'id>>,
    /// Routes are stored in a two-dimensional vector, grouped by destination node.
    _phantom: PhantomData<fn(&'id (), D, NM, CM)>,
}

impl<'id, D: FindableDest<'id, NM, CM>, NM: NodeManager, CM: ContactManager>
    RoutingTable<'id, D, NM, CM>
{
    /// Creates an empty routing table.
    pub fn new() -> Self {
        Self {
            cache: BTreeMap::new(),

            _phantom: PhantomData,
        }
    }
}
impl<'id, NM: NodeManager, CM: ContactManager, D: FindableDest<'id, NM, CM>>
    From<(&Multigraph<'id, NM, CM>, ())> for RoutingTable<'id, D, NM, CM>
{
    fn from(_value: (&Multigraph<'id, NM, CM>, ())) -> Self {
        Self::new()
    }
}

impl<'id, NM: NodeManager, CM: ContactManager, D: FindableDest<'id, NM, CM>>
    PathsStorage<'id, NM, CM, D> for RoutingTable<'id, D, NM, CM>
{
    //TODO:
    //this is technically unsound (the user could, in theory, get a pathfinding output and modify it by hand to create cycles, in wich case any reasonable execution would not terminate / OOM), but is technically UB.
    //This could be circumvented by replacing half of the nice PathFinding api with a new PathFindingMut one.
    fn select<'a>(
        &'a mut self,
        bundle: &Bundle,
        destination: &D,
        route_time: crate::types::Date,
        _curr_time: Option<crate::types::Date>,
        multigraph: &Multigraph<'id, NM, CM>,
    ) -> Result<Option<crate::pathfinding::PathFindingOutput<'id, 'a>>, ASABRError> {
        match destination.to_id(multigraph) {
            None => Ok(None),
            Some(id) => match self.cache.get_mut(&(id, bundle.priority)) {
                None => Ok(None),
                Some(paths) => {
                    if unsafe { destination.validate(paths, route_time, bundle, multigraph) } {
                        Ok(Some(PathFindingOutput::from(&mut **paths)))
                    } else {
                        Ok(None)
                    }
                }
            },
        }
    }

    fn store<'a>(
        &'a mut self,
        tree: crate::pathfinding::PathFindingOutput<'id, 'a>,
        destination: &D,
        bundle: &Bundle,
        _route_time: crate::types::Date,
        _curr_time: Option<crate::types::Date>,
        multigrap: &Multigraph<'id, NM, CM>,
    ) -> crate::pathfinding::PathFindingOutput<'id, 'a> {
        match destination.to_id(multigrap) {
            None => tree,
            Some(id) => {
                let entry = self.cache.entry((id, bundle.priority));
                let entry = entry.insert_entry(tree.into_owned()).into_mut();
                PathFindingOutput::from(entry.as_mut())
            }
        }
    }
}
