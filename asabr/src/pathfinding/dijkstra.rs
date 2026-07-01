extern crate alloc;
use core::marker::PhantomData;

use alloc::vec;

use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    distance::{Distance, prio_queue::PrioQueue},
    multigraph::{Multigraph, NodeRef, RNodeRef},
    node_manager::NodeManager,
    pathfinding::{PathFindingOutput, Pathfinding, destination::Destination, try_make_hop},
    paths::PathFragment,
    types::Date,
};

/// Trait defining a custom DisktraWorkspace.
/// implementing Pathfinding for T can then be done simply using the disktra function.
pub trait DijkstraWorkspace<'id, NM: NodeManager, CM: ContactManager> {
    /// Initialise this Workspace
    fn new(graph: &Multigraph<'id, NM, CM>) -> Self;
    /// Convert self into a (static, aka vector form) pathfinding output
    fn into_pathfinding_output<'a>(self) -> PathFindingOutput<'id, 'a>;
    /// Try to insert a new (better ?) path to a node in self.
    /// If the insert is sucessfull, return a suitable ViaRef to refer to the proposition.
    fn try_insert(
        &mut self,
        proposition: PathFragment<'id>,
        node: NodeRef<'id>,
        graph: &Multigraph<'id, NM, CM>,
        bundle: &Bundle,
    ) -> Option<usize>;
    /// Check if it is usefull to consider new paths to this node.
    fn node_check(&mut self, node: NodeRef<'id>, graph: &Multigraph<'id, NM, CM>) -> bool;
    /// Is this path still relevant, and is it to a new node ?
    /// Also, return the previous node on path if it exist
    fn poped_relevant_new(
        &mut self,
        frag: PathFragment<'id>,
        node: NodeRef<'id>,
        viaref: usize,
    ) -> (bool, bool, Option<RNodeRef<'id>>);
}

pub fn dijkstra<
    'id,
    'a,
    NM: NodeManager,
    CM: ContactManager,
    W: DijkstraWorkspace<'id, NM, CM>,
    D: Distance<NM, CM>,
    De: Destination<'id>,
>(
    multigraph: &mut Multigraph<'id, NM, CM>,
    current_time: Date,
    source: RNodeRef<'id>,
    bundle: &Bundle,
    dest: &mut De,
    prune_time: Option<Date>,
) -> Option<PathFindingOutput<'id, 'a>> {
    let mut work_area = W::new(multigraph);

    let mut prioqueue = PrioQueue::<'_, D, NM, CM, _>::with_capacity(multigraph.get_rnode_count());

    let mut reachable: usize = 1;
    let mut reached: usize = 0;

    let mut reachables = vec![false; multigraph.get_rnode_count()];
    let mut reachables_v = vec![false; multigraph.get_vnode_count()];
    reachables[usize::from(source)] = true;

    let init_path = PathFragment::new_start(current_time, source);
    let viaref = work_area.try_insert(init_path, NodeRef::R(source), multigraph, bundle)?;

    prioqueue.insert((init_path, (viaref, None)), multigraph, bundle);

    while reachable > reached
        && let Some((path, (viaref, isvnode))) = prioqueue.pop_min(multigraph, bundle)
    {
        let node = match isvnode {
            Some(vnoderef) => NodeRef::V(vnoderef),
            None => NodeRef::R(path.rx_node),
        };
        let (relevant, new, previous_node) = work_area.poped_relevant_new(path, node, viaref);

        if new {
            reached += 1;
            if dest.now_reached(node) {
                break;
            }
        }

        if isvnode.is_none() && relevant {
            let node = path.rx_node;
            let (current_node, iter_r, iter_v) = multigraph.iter_iter_contacts(node, prune_time);

            for (neighbor, _, contacts) in iter_r {
                if !work_area.node_check(NodeRef::R(neighbor), multigraph) {
                    continue;
                }

                let delay = current_time;
                match previous_node {
                    None => current_time,
                    Some(tx_node) => current_node.manager.delay(
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
                    delay,
                    contacts.map(|(ctref, ct)| (node, &current_node.manager, ctref, ct)),
                    previous_node,
                ) {
                    if !reachables[usize::from(neighbor)] {
                        reachable += 1;
                        reachables[usize::from(neighbor)] = true
                    }
                    if let Some(viaref) =
                        work_area.try_insert(path, NodeRef::R(neighbor), multigraph, bundle)
                    {
                        prioqueue.insert((path, (viaref, None)), multigraph, bundle);
                    }
                }
            }

            for (vnoderef, contacts) in iter_v {
                if dest.is_useful(vnoderef) {
                    let delay = current_time;
                    match previous_node {
                        None => current_time,
                        Some(tx_node) => current_node.manager.delay(
                            bundle,
                            path.arrival_time,
                            tx_node.into(),
                            multigraph.vnode_id(vnoderef),
                        ),
                    };
                    if let Some(path) = try_make_hop(
                        multigraph,
                        (&path, viaref),
                        bundle,
                        node,
                        delay,
                        contacts.map(|(rre, rno, ctre, ct)| (rre, &rno.manager, ctre, ct)),
                        previous_node,
                    ) {
                        if !reachables_v[usize::from(vnoderef)] {
                            reachable += 1;
                            reachables_v[usize::from(vnoderef)] = true
                        }
                        if let Some(viaref) =
                            work_area.try_insert(path, NodeRef::V(vnoderef), multigraph, bundle)
                        {
                            prioqueue.insert((path, (viaref, Some(vnoderef))), multigraph, bundle);
                        }
                    }
                }
            }
        }
    }
    Some(work_area.into_pathfinding_output())
}

#[derive(Default)]
pub struct Disktra<W, D> {
    _phantom: PhantomData<fn(W, D)>,
}

impl<'id, W, D, NM, CM, De: Destination<'id>> Pathfinding<'id, NM, CM, De> for Disktra<W, D>
where
    W: DijkstraWorkspace<'id, NM, CM>,
    D: Distance<NM, CM>,
    CM: ContactManager,
    NM: NodeManager,
{
    fn find_path<'a>(
        &'a mut self,
        multigraph: &mut Multigraph<'id, NM, CM>,
        current_time: Date,
        source: RNodeRef<'id>,
        bundle: &Bundle,
        dest: &mut De,
        prune_time: Option<Date>,
    ) -> Result<Option<PathFindingOutput<'id, 'a>>, crate::errors::ASABRError> {
        Ok(dijkstra::<NM, CM, W, D, De>(
            multigraph,
            current_time,
            source,
            bundle,
            dest,
            prune_time,
        ))
    }
}

impl<W, D> Disktra<W, D> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}
