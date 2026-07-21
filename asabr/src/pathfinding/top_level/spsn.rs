use core::marker::PhantomData;

use crate::{
    contact_manager::ContactManager,
    multigraph::Multigraph,
    node_manager::NodeManager,
    pathfinding::{
        Pathfinding,
        destination::{All, FindableDest},
    },
    route_storage::{Cached, Guarded},
};

/// A structure representing the Shortest Path with Safety Nodes (SPSN) algorithm.
///
/// This struct handles routing logic and pathfinding, utilizing stored routes
/// and ensuring that the routing process adheres to specified safety and priority constraints.
///
/// # Type Parameters
/// - prio_count: the number of priority to handle in the guard. Set to 1 to ignore priority.
/// - `NM`: A type that implements the `NodeManager` trait, responsible for managing the
///   network's nodes and their interactions.
/// - `CM`: A type that implements the `ContactManager` trait, handling contact points and
///   communication schedules within the network.
/// - `P`: A type that implements the `Pathfinding<NM, CM>` trait, responsible for computing optimal paths.
pub type Spsn<'id, const PRIO_COUNT: usize, NM, CM, P, S, D> =
    Guarded<'id, PRIO_COUNT, Cached<'id, S, AlwaysAll<'id, P, NM, CM, D>, NM, CM, D>, D, NM, CM>;

pub struct AlwaysAll<
    'id,
    P: Pathfinding<'id, NM, CM, All>,
    NM: NodeManager,
    CM: ContactManager,
    D: FindableDest<'id, NM, CM> + ?Sized,
> where
    All: FindableDest<'id, NM, CM>,
{
    pathfinder: P,
    _phantom: PhantomData<fn(&'id (), NM, CM, D)>,
}

impl<
    'id,
    P: Pathfinding<'id, NM, CM, All>,
    NM: NodeManager,
    CM: ContactManager,
    D: FindableDest<'id, NM, CM>,
> Pathfinding<'id, NM, CM, D> for AlwaysAll<'id, P, NM, CM, D>
where
    All: FindableDest<'id, NM, CM>,
{
    fn find_path<'a>(
        &'a mut self,
        multigraph: &mut crate::multigraph::Multigraph<'id, NM, CM>,
        routing_time: crate::types::Date,
        source: crate::multigraph::INodeRef<'id>,
        bundle: &crate::bundle::Bundle,
        _destination: &mut D,
        prune_time: Option<crate::types::Date>,
    ) -> Result<Option<crate::pathfinding::PathFindingOutput<'id, 'a>>, crate::errors::ASABRError>
    {
        self.pathfinder.find_path(
            multigraph,
            routing_time,
            source,
            bundle,
            &mut All,
            prune_time,
        )
    }
}

impl<
    'id,
    P: Pathfinding<'id, NM, CM, All>,
    NM: NodeManager,
    CM: ContactManager,
    D: FindableDest<'id, NM, CM> + ?Sized,
> AlwaysAll<'id, P, NM, CM, D>
where
    All: FindableDest<'id, NM, CM>,
{
    pub fn new(pathfinder: P) -> Self {
        Self {
            pathfinder,
            _phantom: PhantomData,
        }
    }
}

impl<
    'id,
    P: Pathfinding<'id, NM, CM, All>,
    NM: NodeManager,
    CM: ContactManager,
    D: FindableDest<'id, NM, CM> + ?Sized,
    T,
> From<(&Multigraph<'id, NM, CM>, T)> for AlwaysAll<'id, P, NM, CM, D>
where
    All: FindableDest<'id, NM, CM>,
    for<'a> (&'a Multigraph<'id, NM, CM>, T): Into<P>,
{
    fn from(value: (&Multigraph<'id, NM, CM>, T)) -> Self {
        Self::new(value.into())
    }
}
