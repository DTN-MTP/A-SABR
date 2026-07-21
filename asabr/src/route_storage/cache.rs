extern crate alloc;

use core::marker::PhantomData;

use ringbuffer::{AllocRingBuffer, RingBuffer};

use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    errors::ASABRError,
    multigraph::Multigraph,
    node_manager::NodeManager,
    pathfinding::{PathFindingOutput, destination::FindableDest},
    types::{Date, Priority},
};

use super::PathsStorage;

/// A cache for storing pathfinding output entries, enabling efficient retrieval and management.
///
/// The `Cache` struct provides a mechanism to store multiple `PathFindingOutput` instances
/// while enforcing limits on the number of entries based on size and priority checks.
#[derive(Debug)]
pub struct TreeCache<'id, NM: NodeManager, CM: ContactManager> {
    cache: AllocRingBuffer<(Priority, PathFindingOutput<'id, 'id>)>,
    _phantom_nm: PhantomData<fn(&'id (), NM, CM)>,
}
impl<'id, NM: NodeManager, CM: ContactManager> TreeCache<'id, NM, CM> {
    //TODO: maybe infer it from multigraph ?
    /// Creates a route cache with the given capacity.
    pub fn new(_multigrap: &crate::multigraph::Multigraph<'id, NM, CM>, capacity: usize) -> Self {
        Self {
            cache: AllocRingBuffer::new(capacity),
            _phantom_nm: PhantomData,
        }
    }
}

impl<'id, NM: NodeManager, CM: ContactManager> From<(&Multigraph<'id, NM, CM>, usize)>
    for TreeCache<'id, NM, CM>
{
    fn from(value: (&Multigraph<'id, NM, CM>, usize)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl<'id, NM: NodeManager, CM: ContactManager, D: FindableDest<'id, NM, CM>>
    PathsStorage<'id, NM, CM, D> for TreeCache<'id, NM, CM>
{
    fn select<'a>(
        &'a mut self,
        bundle: &Bundle,
        destination: &D,
        route_time: Date,
        _curr_time: Option<Date>,
        multigraph: &crate::multigraph::Multigraph<'id, NM, CM>,
    ) -> Result<Option<PathFindingOutput<'id, 'a>>, ASABRError> {
        for (prio, entry) in self.cache.iter_mut().rev() {
            if bundle.priority == *prio
                && unsafe { destination.validate(entry, route_time, bundle, multigraph) }
            {
                return Ok(Some(PathFindingOutput::from(&mut **entry)));
            }
        }
        Ok(None)
    }

    fn store<'a>(
        &'a mut self,
        tree: PathFindingOutput<'id, 'a>,
        _destination: &D,
        bundle: &Bundle,
        _route_time: crate::types::Date,
        _curr_time: Option<crate::types::Date>,
        _multigraph: &crate::multigraph::Multigraph<'id, NM, CM>,
    ) -> PathFindingOutput<'id, 'a> {
        self.cache.enqueue((bundle.priority, tree.into_owned()));
        PathFindingOutput::from(&mut *self.cache.back_mut().unwrap().1)
    }
}
