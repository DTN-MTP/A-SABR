use core::{cmp::Ordering, marker::PhantomData};

use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    multigraph::Multigraph,
    node_manager::{NodeHeuristic, NodeManager},
    pathfinding::{HybridParentingOrd, destination::FindableDest},
    paths::PathFragment,
};

use super::Distance;

/// Wrap a `Distance` metric `D` with an A* heuristic: order `PathFragment`s
/// by `f = g + h` instead of `g` alone. `g` comes from what `D` already reads of `recv`.
/// `h` comes from `NodeHeuristic::get_heuristic`
#[derive(Debug, Default)]
pub struct AStar<D> {
    _phantom: PhantomData<D>,
}

impl<
    'id,
    NM: NodeManager + NodeHeuristic,
    CM: ContactManager,
    D: Distance<'id, NM, CM, De>,
    De: FindableDest<'id, NM, CM>,
> Distance<'id, NM, CM, De> for AStar<D>
{
    /// Paths reaching the same node are compared by `D` alone: the heuristic can
    /// depend on the arrival time, so comparing by `g + h` could keep a later arrival.
    #[inline(always)]
    fn cmp(
        first: &PathFragment<'id>,
        second: &PathFragment<'id>,
        graph: &Multigraph<'id, NM, CM>,
        bundle: &Bundle,
        destination: &De,
    ) -> Ordering {
        D::cmp(first, second, graph, bundle, destination)
    }

    /// The queue is ordered by `f = g + h`. Ties on `f` go to the smaller `g` (via `D`),
    /// otherwise a time-dependent heuristic can expand a node from a non-optimal path.
    #[inline(always)]
    fn cmp_queue(
        first: &PathFragment<'id>,
        second: &PathFragment<'id>,
        graph: &Multigraph<'id, NM, CM>,
        bundle: &Bundle,
        destination: &De,
    ) -> Ordering {
        let Some(target) = destination.to_id(graph) else {
            return D::cmp_queue(first, second, graph, bundle, destination);
        };
        let target_nodeid = graph.routable_index_to_nodeid(target);

        let f1 = first
            .recv
            .end
            .saturating_add(NM::get_heuristic(first, graph, target_nodeid));
        let f2 = second
            .recv
            .end
            .saturating_add(NM::get_heuristic(second, graph, target_nodeid));

        f1.cmp(&f2)
            .then_with(|| D::cmp_queue(first, second, graph, bundle, destination))
    }
}

impl<NM: NodeManager, CM: ContactManager, D: HybridParentingOrd<NM, CM>> HybridParentingOrd<NM, CM>
    for AStar<D>
{
    #[inline(always)]
    fn keep_both<'id>(
        first: &PathFragment<'id>,
        second: &PathFragment<'id>,
        graph: &Multigraph<'id, NM, CM>,
        bundle: &Bundle,
    ) -> bool {
        // If both paths should be kept is about the accumulated cost, the heuristic doesn't affect.
        D::keep_both(first, second, graph, bundle)
    }
}
