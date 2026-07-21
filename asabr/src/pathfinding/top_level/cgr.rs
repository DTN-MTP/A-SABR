use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    errors::ASABRError,
    multigraph::{INodeRef, Multigraph},
    node_manager::NodeManager,
    pathfinding::{Pathfinding, destination::FindableDest},
    route_storage::PathsStorage,
    types::Date,
};
extern crate alloc;

use core::marker::PhantomData;

/// The legacy, not recomended routing algorithm. See VolCgr or SPSN for better alternative.
/// The sub Pathfinder must be a Suppressor (or similar to it) in the sense that repeated invocation find different paths
pub struct Cgr<
    'id,
    NM: NodeManager,
    CM: ContactManager,
    P: Pathfinding<'id, NM, CM, D>,
    S: PathsStorage<'id, NM, CM, D>,
    D: FindableDest<'id, NM, CM>,
> {
    storage: S,
    pathfinder: P,

    _phantom: PhantomData<fn(&'id (), NM, CM, D)>,
}

impl<
    'id,
    NM: NodeManager,
    CM: ContactManager,
    P: Pathfinding<'id, NM, CM, D>,
    S: PathsStorage<'id, NM, CM, D>,
    D: FindableDest<'id, NM, CM>,
> Pathfinding<'id, NM, CM, D> for Cgr<'id, NM, CM, P, S, D>
{
    fn find_path(
        &mut self,
        multigraph: &mut Multigraph<'id, NM, CM>,
        routing_time: Date,
        source: INodeRef<'id>,
        bundle: &Bundle,
        destination: &mut D,
        prune_time: Option<Date>,
    ) -> Result<Option<crate::pathfinding::PathFindingOutput<'id, '_>>, ASABRError> {
        // Concurent uses of copy validated by polonius
        let copy = &raw mut self.storage;
        if let ret @ (Ok(Some(_)) | Err(_)) = unsafe { copy.as_mut_unchecked() }.select(
            bundle,
            destination,
            routing_time,
            prune_time,
            multigraph,
        ) {
            return ret;
        }
        let mut bundle_copy = bundle.clone();
        bundle_copy.size = 0;
        bundle_copy.priority = 1;

        loop {
            match self.pathfinder.find_path(
                multigraph,
                routing_time,
                source,
                &bundle_copy,
                destination,
                prune_time,
            ) {
                Ok(None) => return Ok(None),
                Err(e) => return Err(e),
                Ok(Some(mut path)) => {
                    // Safety: NOT SAFE
                    // TODO: make it safe
                    if unsafe { destination.validate(&mut path, routing_time, bundle, multigraph) }
                    {
                        return Ok(Some(unsafe { copy.as_mut_unchecked() }.store(
                            path.into_owned(),
                            destination,
                            bundle,
                            routing_time,
                            prune_time,
                            multigraph,
                        )));
                    }
                }
            }
        }
    }
}
impl<
    'id,
    NM: NodeManager,
    CM: ContactManager,
    P: Pathfinding<'id, NM, CM, D>,
    S: PathsStorage<'id, NM, CM, D>,
    D: FindableDest<'id, NM, CM>,
> Cgr<'id, NM, CM, P, S, D>
{
    /// Creates a CGR router from an inner pathfinder and storage backend.
    pub fn new(pathfinder: P, storage: S, _graph: &Multigraph<'id, NM, CM>) -> Self {
        Self {
            pathfinder,
            storage,
            _phantom: PhantomData,
        }
    }
}

impl<
    'id,
    NM: NodeManager,
    CM: ContactManager,
    P: Pathfinding<'id, NM, CM, D>,
    D: FindableDest<'id, NM, CM>,
    S: PathsStorage<'id, NM, CM, D>,
    A,
    B,
> From<(&Multigraph<'id, NM, CM>, (A, B))> for Cgr<'id, NM, CM, P, S, D>
where
    for<'a> (&'a Multigraph<'id, NM, CM>, A): Into<P>,
    for<'a> (&'a Multigraph<'id, NM, CM>, B): Into<S>,
{
    fn from(value: (&Multigraph<'id, NM, CM>, (A, B))) -> Self {
        Self::new(
            (value.0, value.1.0).into(),
            (value.0, value.1.1).into(),
            value.0,
        )
    }
}
