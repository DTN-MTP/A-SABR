use core::{cmp::Ordering, marker::PhantomData};

use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    multigraph::Multigraph,
    node_manager::NodeManager,
    pathfinding::{HybridParentingOrd, destination::FindableDest},
    paths::PathFragment,
};

use super::Distance;

/// Wrap a `Distance` metric `D` with an A* heuristic: order `PathFragment`s
/// by `f = g + h` instead of `g` alone. `g` comes from what `D` already reads of `recv`.
/// `h` comes from `NodeManager::heuristic_delay_to`
#[derive(Debug, Default)]
pub struct AStar<D> {
    _phantom: PhantomData<D>,
}

impl<
    'id,
    NM: NodeManager,
    CM: ContactManager,
    D: Distance<'id, NM, CM, De>,
    De: FindableDest<'id, NM, CM>,
> Distance<'id, NM, CM, De> for AStar<D>
{
    #[inline(always)]
    fn cmp(
        first: &PathFragment<'id>,
        second: &PathFragment<'id>,
        graph: &Multigraph<'id, NM, CM>,
        bundle: &Bundle,
        destination: &De,
    ) -> Ordering {
        let Some(target) = destination.to_id(graph) else {
            return D::cmp(first, second, graph, bundle, destination);
        };
        let h1 = graph[first.rx_node].manager.heuristic_delay_to(target);
        let h2 = graph[second.rx_node].manager.heuristic_delay_to(target);
        let mut adj_first = first.clone();
        let mut adj_second = second.clone();
        adj_first.recv.start = adj_first.recv.start.saturating_add(h1);
        adj_first.recv.end = adj_first.recv.end.saturating_add(h1);
        adj_second.recv.start = adj_second.recv.start.saturating_add(h2);
        adj_second.recv.end = adj_second.recv.end.saturating_add(h2);
        D::cmp(&adj_first, &adj_second, graph, bundle, destination)
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
