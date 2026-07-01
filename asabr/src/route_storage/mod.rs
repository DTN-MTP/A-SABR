extern crate alloc;
use core::marker::PhantomData;

use alloc::collections::BTreeMap;

pub mod cache;
pub mod table;

use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    errors::ASABRError,
    multigraph::{Multigraph, NodeRef, RNodeRef},
    node_manager::NodeManager,
    pathfinding::{PathFindingOutput, Pathfinding, destination::Destination},
    types::{Date, Volume},
};

/// A trait for managing tree storage and retrieval.
///
/// This trait defines methods for loading and storing pathfinding output
/// related to routes in a routing system. Implementers of this trait must
/// provide their own logic for handling route data.
pub trait PathsStorage<'id, NM: NodeManager, CM: ContactManager> {
    /// Loads the pathfinding output for a specific bundle, considering excluded nodes.
    ///
    /// # Parameters
    ///
    /// * `bundle` - A reference to the `Bundle` containing routing information.
    /// * `curr_time` - The current time.
    /// * `excluded_nodes_sorted` - A sorted vector of `NodeID`s representing nodes to exclude from pathfinding.
    ///
    /// # Returns
    ///
    /// * `Result<(Option<Rc<RefCell<PathFindingOutput<NM, CM>>>>, Option<Vec<NodeID>>), ASABRError>` - An optional reference-counted and mutable reference
    ///   to the `PathFindingOutput` if it exists; otherwise, returns `None`.
    fn select<'a>(
        &'a mut self,
        bundle: &Bundle,
        route_time: Date,
        curr_time: Option<Date>,
        multigraph: &Multigraph<'id, NM, CM>,
    ) -> Result<Option<PathFindingOutput<'id, 'a>>, ASABRError>;

    /// Stores the pathfinding output tree for future use, and return it (as reference probably)
    ///
    /// # Parameters
    /// * `bundle` - A bundle copy for which the tree was created.
    /// * `tree` - A reference-counted mutable reference to the `PathfindingOutput` to store.
    fn store<'a>(
        &'a mut self,
        bundle: &Bundle,
        tree: PathFindingOutput<'id, '_>,
    ) -> PathFindingOutput<'id, 'a>;
}

pub struct NoStorage;

impl<'id, NM: NodeManager, CM: ContactManager> PathsStorage<'id, NM, CM> for NoStorage {
    fn select<'a>(
        &'a mut self,
        _bundle: &Bundle,
        _route_time: Date,
        _curr_time: Option<Date>,
        _multigraph: &Multigraph<'id, NM, CM>,
    ) -> Result<Option<PathFindingOutput<'id, 'a>>, ASABRError> {
        Ok(None)
    }
    fn store<'a>(
        &'a mut self,
        _bundle: &Bundle,
        tree: PathFindingOutput<'id, '_>,
    ) -> PathFindingOutput<'id, 'a> {
        tree.clone()
    }
}

pub struct Cached<
    'id,
    S: PathsStorage<'id, NM, CM>,
    P: Pathfinding<'id, NM, CM, D>,
    NM: NodeManager,
    CM: ContactManager,
    D: Destination<'id>,
> {
    cache: S,
    pathfinder: P,
    _phantom: PhantomData<fn(&'id (), NM, CM, D)>,
}

impl<
    'id,
    S: PathsStorage<'id, NM, CM>,
    P: Pathfinding<'id, NM, CM, D>,
    NM: NodeManager,
    CM: ContactManager,
    D: Destination<'id>,
> Pathfinding<'id, NM, CM, D> for Cached<'id, S, P, NM, CM, D>
{
    fn find_path<'a>(
        &'a mut self,
        multigraph: &mut Multigraph<'id, NM, CM>,
        routing_time: Date,
        source: crate::multigraph::RNodeRef<'id>,
        bundle: &Bundle,
        destination: &mut D,
        prune_time: Option<Date>,
    ) -> Result<Option<PathFindingOutput<'id, 'a>>, ASABRError> {
        match self
            .cache
            .select(bundle, routing_time, prune_time, multigraph)
        {
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
                    Ok(Some(path)) => Ok(Some(self.cache.store(bundle, path))),
                }
            }
        }
    }
}

impl<
    'id,
    S: PathsStorage<'id, NM, CM>,
    P: Pathfinding<'id, NM, CM, D>,
    NM: NodeManager,
    CM: ContactManager,
    D: Destination<'id>,
> Cached<'id, S, P, NM, CM, D>
{
    pub fn new(storage: S, pathfinder: P) -> Self {
        Self {
            cache: storage,
            pathfinder,
            _phantom: PhantomData,
        }
    }
}

/// Tells us a destination is guardable, necessary to use it with the guarded pathfinder.
pub trait Guardable<'id>: Destination<'id> {
    /// id to guard this destination on
    fn as_id(&self, graph: &Multigraph<'id, impl NodeManager, impl ContactManager>) -> usize;
}

/// A Guard to avoid searching a path when useless. Bundles prio will be capped at prio_count (set to 1 to ignore bundles priorities)
#[derive(Debug, Default)]
pub struct Guard<'id, G: Guardable<'id>, const PRIO_COUNT: usize> {
    limits: BTreeMap<usize, [Option<Volume>; PRIO_COUNT]>,
    _phantom: PhantomData<fn(&'id (), G)>,
}

impl<'id, const PRIO_COUNT: usize, G: Guardable<'id>> Guard<'id, G, PRIO_COUNT> {
    pub fn new() -> Self {
        Self {
            limits: BTreeMap::new(),
            _phantom: PhantomData,
        }
    }
    pub fn set_limit(
        &mut self,
        bundle: &Bundle,
        dest: &G,
        graph: &Multigraph<'id, impl NodeManager, impl ContactManager>,
    ) {
        let place = self
            .limits
            .entry(dest.as_id(graph))
            .or_insert([None; PRIO_COUNT]);
        for place in place.iter_mut().take(bundle.priority as usize + 1) {
            *place = Some(place.map_or(bundle.size, |old| old.min(bundle.size)))
        }
    }
    pub fn abort(
        &self,
        bundle: &Bundle,
        dest: &G,
        graph: &Multigraph<'id, impl NodeManager, impl ContactManager>,
    ) -> bool {
        let place = &self.limits[&dest.as_id(graph)];
        place[(PRIO_COUNT - 1).min(bundle.priority as usize)]
            .is_some_and(|limit| limit <= bundle.size)
    }
}

/// A guarded PathFinder. Once a node is marked as unreachable, never try to find a path to it again.
/// The destination must implement Guardable
pub struct Guarded<
    'id,
    const PRIO_COUNT: usize,
    P: Pathfinding<'id, NM, CM, D>,
    D: Destination<'id> + Guardable<'id>,
    NM: NodeManager,
    CM: ContactManager,
> {
    finder: P,
    guard: Guard<'id, D, PRIO_COUNT>,
    _phantom: PhantomData<fn(CM, NM)>,
}

impl<
    'id,
    const PRIO_COUNT: usize,
    P: Pathfinding<'id, NM, CM, D>,
    D: Destination<'id> + Guardable<'id>,
    NM: NodeManager,
    CM: ContactManager,
> Guarded<'id, PRIO_COUNT, P, D, NM, CM>
{
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
    const PRIO_COUNT: usize,
    P: Pathfinding<'id, NM, CM, D>,
    D: Destination<'id> + Guardable<'id>,
    NM: NodeManager,
    CM: ContactManager,
> Pathfinding<'id, NM, CM, D> for Guarded<'id, PRIO_COUNT, P, D, NM, CM>
{
    fn find_path<'a>(
        &'a mut self,
        multigraph: &mut Multigraph<'id, NM, CM>,
        routing_time: Date,
        source: crate::multigraph::RNodeRef<'id>,
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

impl<'id> Guardable<'id> for NodeRef<'id> {
    fn as_id(&self, graph: &Multigraph<'id, impl NodeManager, impl ContactManager>) -> usize {
        graph.into_usize(*self)
    }
}
impl<'id> Guardable<'id> for RNodeRef<'id> {
    fn as_id(&self, _graph: &Multigraph<'id, impl NodeManager, impl ContactManager>) -> usize {
        (*self).into()
    }
}
