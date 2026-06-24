extern crate alloc;
use core::marker::PhantomData;

use alloc::vec;

use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    distance::{Distance, prio_queue::PrioQueue},
    multigraph::{Multigraph, NodeRef, RNodeRef},
    node_manager::NodeManager,
    pathfinding::{PathFindingOutput, Pathfinding, try_make_hop},
    paths::{PathFragment, ViaHop},
    types::{Date, NodeID},
};

/// Trait defining a custom DisktraWorkspace.
/// implementing Pathfinding for T can then be done simply using the disktra function.
pub trait DisktraWorkspace<'id, NM: NodeManager, CM: ContactManager> {
    /// Initialise this Workspace
    fn new(graph: &Multigraph<'id, NM, CM>) -> Self;
    /// Convert self into a (static, aka vector form) pathfinding output
    fn into_pathfinding_output(self) -> PathFindingOutput<'id, 'static>;
    /// Try to insert a new (better ?) path to a node in self.
    /// If the insert is sucessfull, return a suitable ViaRef to refer to the proposition.
    /// Also return wether this node is a newly reached one or not
    fn try_insert(
        &mut self,
        proposition: PathFragment<'id>,
        actual_node: RNodeRef<'id>,
        graph: &Multigraph<'id, NM, CM>,
        bundle: &Bundle,
    ) -> (Option<usize>, bool);
    /// Check if it is usefull to consider new paths to this node.
    fn node_check(&mut self, node: RNodeRef<'id>) -> bool;
    /// Check if it is usefull to add this path proposition in disktra prioqueue.
    fn fragment_check(
        &mut self,
        proposition: PathFragment<'id>,
        dest_node: RNodeRef<'id>,
        graph: &Multigraph<'id, NM, CM>,
        bundle: &Bundle,
    ) -> bool;
}

pub fn disktra<
    'id,
    NM: NodeManager,
    CM: ContactManager,
    W: DisktraWorkspace<'id, NM, CM>,
    D: Distance<NM, CM>,
>(
    multigraph: &mut Multigraph<'id, NM, CM>,
    current_time: Date,
    source: RNodeRef<'id>,
    bundle: &Bundle,
) -> PathFindingOutput<'id, 'static> {
    let mut work_area = W::new(multigraph);

    let mut prioqueue = PrioQueue::<'_, D, NM, CM, ()>::with_capacity(multigraph.get_rnode_count());

    let mut reachable: usize = 1;
    let mut reached: usize = 0;

    let mut reachables = vec![false; multigraph.get_rnode_count()];
    reachables[NodeID::from(source) as usize] = true;

    prioqueue.insert(
        ((PathFragment::new_start(current_time), source), ()),
        multigraph,
        bundle,
    );

    while reachable > reached
        && let Some(((path, node), ())) = prioqueue.pop_min(multigraph, bundle)
    {
        let (viaref, new) = work_area.try_insert(path, node, multigraph, bundle);
        if new {
            reached += 1
        }
        if let Some(viaref) = viaref {
            let (current_node, iter) = multigraph.iter_iter_contacts(node);
            for (neighbor, a, contacts) in iter {
                if !work_area.node_check(neighbor) {
                    continue;
                }

                let delay = match path.via {
                    None => current_time,
                    Some(ViaHop { tx_node, .. }) => current_node.manager.delay(
                        bundle,
                        path.arrival_time,
                        tx_node.into(),
                        neighbor.into(),
                    ),
                };
                if let Some(path) = try_make_hop(
                    multigraph,
                    (&path, viaref),
                    bundle,
                    node,
                    neighbor,
                    delay,
                    contacts,
                ) {
                    if !reachables[NodeID::from(neighbor) as usize] {
                        reachable += 1;
                        reachables[NodeID::from(neighbor) as usize] = true
                    }
                    if work_area.fragment_check(path, neighbor, multigraph, bundle) {
                        prioqueue.insert(((path, neighbor), ()), multigraph, bundle);
                    }
                }
            }
        }
    }

    work_area.into_pathfinding_output()
}

pub struct Disktra<W, D> {
    _phantom: PhantomData<fn(W, D)>,
}

impl<'id, W, D, NM, CM> Pathfinding<'id, NM, CM> for Disktra<W, D>
where
    W: DisktraWorkspace<'id, NM, CM>,
    D: Distance<NM, CM>,
    CM: ContactManager,
    NM:NodeManager
{
    fn new(id: generativity::Guard, multigraph: &Multigraph<'id, NM, CM>) -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
    fn find_path<'a>(
        &'a mut self,
        multigraph: &mut Multigraph<'id, NM, CM>,
        current_time: Date,
        source: RNodeRef<'id>,
        bundle: &Bundle,
    ) -> Result<Option<PathFindingOutput<'id, 'a>>, crate::errors::ASABRError> {
        Ok(Some(disktra::<'id, NM, CM, W, D>(
            multigraph,
            current_time,
            source,
            bundle,
        )))
    }
}
