extern crate alloc;
use core::marker::PhantomData;

use alloc::collections::BTreeMap;

/// Ring-buffer route cache storage.
pub mod cache;
/// Destination and priority keyed route table storage.
pub mod table;

use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    errors::ASABRError,
    multigraph::{INodeRef, Multigraph},
    node_manager::NodeManager,
    pathfinding::{PathFindingOutput, Pathfinding, destination::FindableDest},
    types::{Date, Volume},
};

/// A trait for managing tree storage and retrieval.
///
/// This trait defines methods for loading and storing pathfinding output
/// related to routes in a routing system. Implementers of this trait must
/// provide their own logic for handling route data.
pub trait PathsStorage<'id, NM: NodeManager, CM: ContactManager, D: FindableDest<'id, NM, CM>> {
    /// Loads a stored pathfinding output for a bundle and destination.
    ///
    /// # Parameters
    ///
    /// * `bundle` - The bundle being routed.
    /// * `destination` - The destination selector.
    /// * `route_time` - The time used for route validation.
    /// * `curr_time` - Optional current time for storage implementations that need it.
    /// * `multigraph` - The graph containing the stored path references.
    ///
    /// # Returns
    ///
    /// A stored `PathFindingOutput` if one is available and still valid.
    fn select<'a>(
        &'a mut self,
        bundle: &Bundle,
        destination: &D,
        route_time: Date,
        curr_time: Option<Date>,
        multigraph: &Multigraph<'id, NM, CM>,
    ) -> Result<Option<PathFindingOutput<'id, 'a>>, ASABRError>;

    /// Stores the pathfinding output tree for future use, and return it (as reference probably)
    ///
    /// # Parameters
    /// * `tree` - The pathfinding output to store.
    /// * `destination` - The destination selector.
    /// * `bundle` - The bundle for which the tree was created.
    fn store<'a>(
        &'a mut self,
        tree: PathFindingOutput<'id, 'a>,
        destination: &D,
        bundle: &Bundle,
        _route_time: crate::types::Date,
        _curr_time: Option<crate::types::Date>,
        multigraph: &Multigraph<'id, NM, CM>,
    ) -> PathFindingOutput<'id, 'a>;
}

/// Path storage implementation that never reuses routes.
pub struct NoStorage;

impl<'id, NM: NodeManager, CM: ContactManager, D: FindableDest<'id, NM, CM>>
    PathsStorage<'id, NM, CM, D> for NoStorage
{
    fn select<'a>(
        &'a mut self,
        _bundle: &Bundle,
        _destination: &D,
        _route_time: Date,
        _curr_time: Option<Date>,
        _multigraph: &Multigraph<'id, NM, CM>,
    ) -> Result<Option<PathFindingOutput<'id, 'a>>, ASABRError> {
        Ok(None)
    }
    fn store<'a>(
        &'a mut self,
        tree: PathFindingOutput<'id, 'a>,
        _destination: &D,
        _bundle: &Bundle,
        _route_time: crate::types::Date,
        _curr_time: Option<crate::types::Date>,
        _multigraph: &Multigraph<'id, NM, CM>,
    ) -> PathFindingOutput<'id, 'a> {
        tree.into_owned()
    }
}

/// Pathfinding wrapper that consults a storage backend before computing paths.
pub struct Cached<
    'id,
    S: PathsStorage<'id, NM, CM, D>,
    P: Pathfinding<'id, NM, CM, D>,
    NM: NodeManager,
    CM: ContactManager,
    D: FindableDest<'id, NM, CM>,
> {
    cache: S,
    pathfinder: P,
    _phantom: PhantomData<fn(&'id (), NM, CM, D)>,
}

impl<
    'id,
    S: PathsStorage<'id, NM, CM, D>,
    P: Pathfinding<'id, NM, CM, D>,
    NM: NodeManager,
    CM: ContactManager,
    D: FindableDest<'id, NM, CM>,
> Pathfinding<'id, NM, CM, D> for Cached<'id, S, P, NM, CM, D>
{
    fn find_path<'a>(
        &'a mut self,
        multigraph: &mut Multigraph<'id, NM, CM>,
        routing_time: Date,
        source: INodeRef<'id>,
        bundle: &Bundle,
        destination: &mut D,
        prune_time: Option<Date>,
    ) -> Result<Option<PathFindingOutput<'id, 'a>>, ASABRError> {
        // Concurent usage validated by polonius
        let copy = &raw mut self.cache;
        match unsafe { copy.as_mut_unchecked() }.select(
            bundle,
            destination,
            routing_time,
            prune_time,
            multigraph,
        ) {
            res @ (Ok(Some(_)) | Err(_)) => res,
            Ok(None) => {
                match self.pathfinder.find_path(
                    multigraph,
                    routing_time,
                    source,
                    bundle,
                    destination,
                    prune_time,
                ) {
                    res @ (Ok(None) | Err(_)) => res,
                    Ok(Some(path)) => Ok(Some(unsafe { copy.as_mut_unchecked() }.store(
                        path,
                        destination,
                        bundle,
                        routing_time,
                        prune_time,
                        multigraph,
                    ))),
                }
            }
        }
    }
}

impl<
    'id,
    S: PathsStorage<'id, NM, CM, D>,
    P: Pathfinding<'id, NM, CM, D>,
    NM: NodeManager,
    CM: ContactManager,
    D: FindableDest<'id, NM, CM>,
> Cached<'id, S, P, NM, CM, D>
{
    /// Creates a cached pathfinder from storage and an inner pathfinder.
    pub fn new(storage: S, pathfinder: P) -> Self {
        Self {
            cache: storage,
            pathfinder,
            _phantom: PhantomData,
        }
    }
}

impl<
    'id,
    S: PathsStorage<'id, NM, CM, D>,
    P: Pathfinding<'id, NM, CM, D>,
    NM: NodeManager,
    CM: ContactManager,
    D: FindableDest<'id, NM, CM>,
    A,
    B,
> From<(&Multigraph<'id, NM, CM>, (A, B))> for Cached<'id, S, P, NM, CM, D>
where
    for<'a> (&'a Multigraph<'id, NM, CM>, A): Into<S>,
    for<'a> (&'a Multigraph<'id, NM, CM>, B): Into<P>,
{
    fn from(value: (&Multigraph<'id, NM, CM>, (A, B))) -> Self {
        Self::new((value.0, value.1.0).into(), (value.0, value.1.1).into())
    }
}

/// A Guard to avoid searching a path when useless. Bundles prio will be capped at prio_count (set to 1 to ignore bundles priorities)
#[derive(Debug, Default)]
pub struct Guard<
    'id,
    D: FindableDest<'id, NM, CM>,
    const PRIO_COUNT: usize,
    NM: NodeManager,
    CM: ContactManager,
> {
    limits: BTreeMap<usize, [Option<Volume>; PRIO_COUNT]>,
    _phantom: PhantomData<fn(&'id (), D, NM, CM)>,
}

impl<
    'id,
    const PRIO_COUNT: usize,
    D: FindableDest<'id, NM, CM>,
    NM: NodeManager,
    CM: ContactManager,
> Guard<'id, D, PRIO_COUNT, NM, CM>
{
    /// Creates an empty guard.
    pub fn new() -> Self {
        Self {
            limits: BTreeMap::new(),
            _phantom: PhantomData,
        }
    }
    /// Records an unsuccessful route size limit for a destination.
    pub fn set_limit(&mut self, bundle: &Bundle, dest: &D, graph: &Multigraph<'id, NM, CM>) {
        let Some(id) = dest.to_id(graph) else {
            return;
        };
        let place = self.limits.entry(id).or_insert([None; PRIO_COUNT]);
        for place in place.iter_mut().take(bundle.priority as usize + 1) {
            *place = Some(place.map_or(bundle.size, |old| old.min(bundle.size)))
        }
    }
    /// Returns whether route search can be skipped for the bundle and destination.
    pub fn abort(&self, bundle: &Bundle, dest: &D, graph: &Multigraph<'id, NM, CM>) -> bool {
        let Some(id) = dest.to_id(graph) else {
            return false;
        };
        match &self.limits.get(&id) {
            None => false,
            Some(place) => place[(PRIO_COUNT - 1).min(bundle.priority as usize)]
                .is_some_and(|limit| limit <= bundle.size),
        }
    }
}

/// A guarded PathFinder. Once a node is marked as unreachable, never try to find a path to it again. (If bundle is larger and less prioritary)
/// Rely on the destination .into_id() implementation
pub struct Guarded<
    'id,
    const PRIO_COUNT: usize,
    P: Pathfinding<'id, NM, CM, D>,
    D: FindableDest<'id, NM, CM>,
    NM: NodeManager,
    CM: ContactManager,
> {
    finder: P,
    guard: Guard<'id, D, PRIO_COUNT, NM, CM>,
    _phantom: PhantomData<fn(CM, NM)>,
}

impl<
    'id,
    const PRIO_COUNT: usize,
    P: Pathfinding<'id, NM, CM, D>,
    D: FindableDest<'id, NM, CM>,
    NM: NodeManager,
    CM: ContactManager,
> Guarded<'id, PRIO_COUNT, P, D, NM, CM>
{
    /// Creates a guarded pathfinder around an inner pathfinder.
    pub fn new(finder: P) -> Self {
        Self {
            finder,
            guard: Guard::new(),
            _phantom: PhantomData,
        }
    }
}

impl<
    'id,
    P: Pathfinding<'id, NM, CM, D>,
    NM: NodeManager,
    CM: ContactManager,
    D: FindableDest<'id, NM, CM>,
    T,
    const PC: usize,
> From<(&Multigraph<'id, NM, CM>, T)> for Guarded<'id, PC, P, D, NM, CM>
where
    for<'a> (&'a Multigraph<'id, NM, CM>, T): Into<P>,
{
    fn from(value: (&Multigraph<'id, NM, CM>, T)) -> Self {
        Self::new(value.into())
    }
}

impl<
    'id,
    const PRIO_COUNT: usize,
    P: Pathfinding<'id, NM, CM, D>,
    D: FindableDest<'id, NM, CM>,
    NM: NodeManager,
    CM: ContactManager,
> Pathfinding<'id, NM, CM, D> for Guarded<'id, PRIO_COUNT, P, D, NM, CM>
{
    fn find_path<'a>(
        &'a mut self,
        multigraph: &mut Multigraph<'id, NM, CM>,
        routing_time: Date,
        source: INodeRef<'id>,
        bundle: &Bundle,
        destination: &mut D,
        prune_time: Option<Date>,
    ) -> Result<Option<PathFindingOutput<'id, 'a>>, ASABRError> {
        if self.guard.abort(bundle, destination, multigraph) {
            Ok(None)
        } else {
            match self.finder.find_path(
                multigraph,
                routing_time,
                source,
                bundle,
                destination,
                prune_time,
            ) {
                ret @ (Ok(Some(_)) | Err(_)) => ret,
                Ok(None) => {
                    self.guard.set_limit(bundle, destination, multigraph);
                    Ok(None)
                }
            }
        }
    }
}
