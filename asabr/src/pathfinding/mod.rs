extern crate alloc;
use alloc::vec::Vec;
use core::{
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::{
    bundle::Bundle,
    contact::Contact,
    contact_manager::{ContactManager, ContactManagerTxData},
    errors::ASABRError,
    multigraph::{ContactRef, INodeRef, Multigraph, RealNodeRef, RoutableNodeRef},
    node_manager::NodeManager,
    parsing::Either,
    pathfinding::destination::FindableDest,
    paths::{PathFragment, ViaHop},
    types::{Date, NodeID, TimeInterval},
};

/// Dijkstra-based pathfinding entry points.
pub mod dijkstra;
/// Dijkstra workspace implementations.
pub mod dijkstra_impl;
pub use dijkstra_impl::*;
/// Destination types used by pathfinders.
pub mod destination;
pub use destination::{All as DestAll, Dest};
#[cfg(feature = "contact_suppression")]
pub mod limiting_contact;
#[cfg(test)]
mod test_helpers;
pub mod top_level;

/// Data structure that holds the results of a pathfinding operation.
///
/// It is a path Tree where the end of the path from the initial node to a node N with nodeID id is self[id as usize].
/// None mean no path.
/// Viaref ParenFrag ar index in the path tree.
/// Access is meant to be done using the deref to slice impl, not matching on the variants, these should be used only to construct a pathfinding output
///
/// # Type Parameters
///
/// * `'id` - Graph identity lifetime.
/// * `'a` - Lifetime of the borrowed or owned path tree.
#[derive(Debug)]
pub struct PathFindingOutput<'id, 'a> {
    path_tree: Either<&'a mut [Option<PathFragment<'id>>], Vec<Option<PathFragment<'id>>>>,
}

impl Clone for PathFindingOutput<'_, '_> {
    fn clone(&self) -> Self {
        Self::from(Vec::from(&**self))
    }
}

impl<'id, 'a> AsRef<[Option<PathFragment<'id>>]> for PathFindingOutput<'id, 'a> {
    fn as_ref(&self) -> &[Option<PathFragment<'id>>] {
        match &self.path_tree {
            Either::Left(l) => l,
            Either::Right(r) => r.as_ref(),
        }
    }
}

impl<'id, 'a> AsRef<PathFindingOutput<'id, 'a>> for PathFindingOutput<'id, 'a> {
    fn as_ref(&self) -> &PathFindingOutput<'id, 'a> {
        self
    }
}

impl<'id, 'a> Deref for PathFindingOutput<'id, 'a> {
    type Target = [Option<PathFragment<'id>>];
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<'id, 'a> AsMut<[Option<PathFragment<'id>>]> for PathFindingOutput<'id, 'a> {
    fn as_mut(&mut self) -> &mut [Option<PathFragment<'id>>] {
        match &mut self.path_tree {
            Either::Left(l) => l,
            Either::Right(r) => r.as_mut(),
        }
    }
}

impl<'id, 'a> DerefMut for PathFindingOutput<'id, 'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<'id, 'a> From<Vec<Option<PathFragment<'id>>>> for PathFindingOutput<'id, 'a> {
    fn from(value: Vec<Option<PathFragment<'id>>>) -> Self {
        Self {
            path_tree: Either::Right(value),
        }
    }
}
impl<'id, 'a> From<&'a mut [Option<PathFragment<'id>>]> for PathFindingOutput<'id, 'a> {
    fn from(value: &'a mut [Option<PathFragment<'id>>]) -> Self {
        Self {
            path_tree: Either::Left(value),
        }
    }
}

impl<'id, 'a> PathFindingOutput<'id, 'a> {
    /// Return the list of hops making this path, if it is still a valid (and detected) one,
    pub fn get_full_path<NM: NodeManager, CM: ContactManager>(
        &self,
        destination: RoutableNodeRef<'id>,
        graph: &Multigraph<'id, NM, CM>,
    ) -> Option<Vec<PathFragment<'id>>> {
        let mut next = self[graph.routable_to_usize(destination)]?;
        let mut r = Vec::with_capacity(next.hop_count as usize + 1);
        r.push(next);
        while let Some(next_via) = next.via {
            next = self[next_via.parent_frag]?;
            r.push(next);
        }
        r.reverse();
        Some(r)
    }
    /// Returns an iterator over a destination path from destination back to source.
    pub fn full_path_rev<NM: NodeManager, CM: ContactManager>(
        &self,
        destination: RoutableNodeRef<'id>,
        graph: &Multigraph<'id, NM, CM>,
    ) -> Option<PathIterator<'id, 'a, &PathFindingOutput<'id, 'a>>> {
        self[graph.routable_to_usize(destination)].map(|_| PathIterator {
            output: self,
            last: Some(graph.routable_to_usize(destination)),
            _phantom: PhantomData,
        })
    }
    pub fn full_path_rev_owned(
        self,
        destination: RoutableNodeRef<'id>,
        graph: &mut Multigraph<'id, impl NodeManager, impl ContactManager>,
    ) -> Option<PathIterator<'id, 'a, PathFindingOutput<'id, 'a>>> {
        self[graph.routable_to_usize(destination)].map(|_| PathIterator {
            output: self,
            last: Some(graph.routable_to_usize(destination)),
            _phantom: PhantomData,
        })
    }
    /// Commits the path to the graph for the given bundle and destination.
    pub fn commit_path_to(
        &self,
        destination: RoutableNodeRef<'id>,
        bundle: &Bundle,
        graph: &mut Multigraph<'id, impl NodeManager, impl ContactManager>,
    ) -> Result<Option<PathFragment<'id>>, ASABRError> {
        match self.full_path_rev(destination, graph) {
            None => Ok(None),
            Some(path) => path.commit(bundle, graph),
        }
    }
    /// Converts the output into an owned pathfinding output.
    pub fn into_owned<'b>(self) -> PathFindingOutput<'id, 'b> {
        let vec = self.as_vec();
        PathFindingOutput {
            path_tree: Either::Right(vec),
        }
    }
    /// Converts the output into a vector of optional path fragments.
    pub fn as_vec(self) -> Vec<Option<PathFragment<'id>>> {
        match self.path_tree {
            Either::Left(value) => value.to_vec(),
            Either::Right(mut vec) => {
                vec.shrink_to_fit();
                vec
            }
        }
    }
    /// Intended for implementor of paths storage
    /// from a mutable access to a stored vec representing a pathfinding output, get mutable access to each of the components of the path to a destination
    ///
    /// # Safety
    /// self paths should have no cycle (wich is true of any reasonable PathfindingOutput not modified by hand outside of the library)
    pub unsafe fn get_path_mut<'b>(
        &'b mut self,
        destination: usize,
    ) -> Option<Vec<&'b mut PathFragment<'id>>> {
        // multiple borrow occur but we assume there can be no cycle, and as such the different borrow are actually all on different cells.
        let storage = &raw mut **self;
        let next = unsafe { &mut (*storage)[destination] }.as_mut()?;
        let mut collect = Vec::with_capacity(next.hop_count as usize + 1);
        let mut next = collect.push_mut(next);
        while let Some(ViaHop { parent_frag, .. }) = next.via {
            next = collect.push_mut(unsafe { &mut (*storage)[parent_frag] }.as_mut()?)
        }
        collect.reverse();
        Some(collect)
    }

    /// Check wether this is still a valid path for the given bundle/destination,
    /// and update it to reflect the correct times
    /// # Safety
    /// self paths should have no cycle (wich is true of any reasonable PathfindingOutput not modified by hand outside of the library)
    pub unsafe fn validate(
        &mut self,
        dest: RoutableNodeRef<'id>,
        time: Date,
        bundle: &Bundle,
        graph: &Multigraph<'id, impl NodeManager, impl ContactManager>,
    ) -> bool {
        let path = unsafe { self.get_path_mut(graph.routable_to_usize(dest)) };
        path.is_some_and(|mut path| {
            let mut last_node = None;
            if let Some(PathFragment {
                recv: arrival_time, ..
            }) = path.first_mut()
            {
                *arrival_time = TimeInterval {
                    start: time,
                    end: time,
                };
            }
            let mut idx = 0;
            while let Ok([fst, snd]) = path.get_disjoint_mut([idx, idx + 1]) {
                if graph[fst.rx_node].info.excluded {
                    return false;
                }
                let time = match last_node {
                    Some(nodeid) => graph[fst.rx_node].manager.process_delay(
                        bundle,
                        fst.recv,
                        nodeid,
                        graph.into_nodeid(snd.rx_node.into()),
                    ),
                    None => time,
                };
                let ct = snd.via.as_mut().unwrap();
                let tx_data = graph[ct.contact].dry_run_tx(time, bundle);
                match tx_data {
                    None => return false,
                    Some(tx_data) => {
                        ct.send = tx_data.send;
                        snd.recv = tx_data.recv;
                    }
                }
                if let Some(nodeid) = last_node
                    && !graph[fst.rx_node].manager.dry_run_retention(
                        bundle,
                        fst.recv,
                        nodeid,
                        ct.send,
                        graph.into_nodeid(snd.rx_node.into()),
                    )
                {
                    return false;
                }
                last_node = Some(graph.into_nodeid(fst.rx_node.into()));
                idx += 1
            }
            if let (Some(last), Some(prev)) = (path.last_mut(), last_node)
                && (!graph[last.rx_node].manager.accept(bundle, last.recv, prev)
                    || graph[last.rx_node].info.excluded)
            {
                return false;
            }
            true
        })
    }
}

/// Iterator over path fragments from destination back to source.
#[derive(Debug, Clone)]
pub struct PathIterator<'id, 'a, T: AsRef<PathFindingOutput<'id, 'a>>> {
    output: T,
    last: Option<usize>,
    _phantom: PhantomData<PathFindingOutput<'id, 'a>>,
}

impl<'id, 'a, T: AsRef<PathFindingOutput<'id, 'a>>> Iterator for PathIterator<'id, 'a, T> {
    type Item = PathFragment<'id>;
    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.last.take()?;
        let frag = self.output.as_ref()[idx]?;
        if let Some(hop) = frag.via {
            self.last = Some(hop.parent_frag)
        }
        Some(frag)
    }
}

impl<'id, 'a, T: AsRef<PathFindingOutput<'id, 'a>>> PathIterator<'id, 'a, T> {
    /// Commit a single destination path, and return the PathFragment of the first hop, or the first error.
    pub fn commit(
        self,
        bundle: &Bundle,
        multigraph: &mut Multigraph<'id, impl NodeManager, impl ContactManager>,
    ) -> Result<Option<PathFragment<'id>>, ASABRError> {
        let mut iter = self.peekable();
        let Some(mut last) = iter.next() else {
            return Ok(None);
        };
        let Some(prev) = iter.peek() else {
            return Ok(None);
        };

        multigraph[last.rx_node].manager.commit(
            bundle,
            last.recv,
            // Not the last => only internal node
            unsafe { prev.rx_node.internal().unwrap_unchecked() }.into(),
            &[],
        )?;
        while let Some(curr) = iter.next() {
            // Not the first => via exist
            let last_via = unsafe { last.via.unwrap_unchecked() };
            multigraph[last_via.contact].schedule_tx(
                ContactManagerTxData {
                    send: last_via.send,
                    recv: last.recv,
                },
                bundle,
            )?;

            if let Some(prev) = iter.peek() {
                let last_nodeid = multigraph.into_nodeid(last.rx_node.into());
                // Not the last => only internal node
                multigraph[unsafe { curr.rx_node.internal().unwrap_unchecked() }]
                    .manager
                    .commit(
                        bundle,
                        curr.recv,
                        // Not the last => only internal node
                        unsafe { prev.rx_node.internal().unwrap_unchecked().into() },
                        // Not the first => via exist
                        &[(last_via.send, last_nodeid)],
                    )?;
                last = curr;
            }
        }
        Ok(Some(last))
    }
}

impl<'id, 'a, T: AsRef<PathFindingOutput<'id, 'a>>> core::fmt::Display for PathIterator<'id, 'a, T>
where
    T: Clone,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut copy = self.clone().peekable();
        if let Some(last) = copy.peek() {
            writeln!(
                f,
                "Route to {} at t={} with {} hop(s):",
                last.rx_node, last.recv.end, last.hop_count
            )?;
        };
        for frag in copy {
            writeln!(
                f,
                "        - Reach node {} at t={} with {} hop(s)",
                frag.rx_node, frag.recv.end, frag.hop_count
            )?;
        }
        Ok(())
    }
}

/// The `Pathfinding` trait provides the interface for implementing a pathfinding algorithm.
/// It requires methods for creating a new instance and determining the next hop in a route.
///
/// # Type Parameters
///
/// * `NM` - A generic type that implements the `NodeManager` trait.
/// * `CM` - A generic type that implements the `ContactManager` trait.
pub trait Pathfinding<
    'id,
    NM: NodeManager,
    CM: ContactManager,
    D: FindableDest<'id, NM, CM> + ?Sized,
>
{
    /// Determines the routing tree in the multigraph for the given bundle.
    /// Populate the routes until the destination is reached.
    /// The bundle will be launched at `routing_time`, and old contacts in the graph may be ellided if they are older than `prune_time`.
    ///
    /// Take into account nodes/contacts marked as excluded in the graph, eg with `Multigraph::mark_excluded`
    ///
    /// # Parameters
    /// * `multigraph` the graph to search into
    /// * `routing_time` - The time at wich the bundle leave the current node.
    /// * `source` - The `RNodeRef` of the source node.
    /// * `bundle` - Ihe `Bundle` being routed.
    /// * `destination` - A templated destination telling the pathfinder when to stop populating the output.
    /// * `prune_time` - Optional cutoff for ignoring old contacts in the graph.
    ///
    /// # Returns
    ///
    /// A `Result<PathFindingOutput<NM, CM>, ASABRError>` containing the results of the pathfinding operation,
    /// or an error if the operation fails.
    fn find_path<'a>(
        &'a mut self,
        multigraph: &mut Multigraph<'id, NM, CM>,
        routing_time: Date,
        source: INodeRef<'id>,
        bundle: &Bundle,
        destination: &mut D,
        prune_time: Option<Date>,
    ) -> Result<Option<PathFindingOutput<'id, 'a>>, ASABRError>;
}
/// Attempts to make a hop (i.e., a transmission between nodes) for the given route stage and bundle,
/// checking potential contacts to determine the best hop.
///
/// # Parameters
///
/// * `graph` - The multigraph we are searching a route into
/// * `last_hop` - The previous PathFragment, and a reference to it
/// * `bundle` - A reference to the `Bundle` that is being routed.
/// * `current_node` - the node the bundle is at and we try to leave
/// * `next_node` - the node we target
/// * `send_time` - the time at wich the paquet should try to be sent
/// * `contacts` - A iterator over potentially suitable contacts. This will try to select the first contact
/// * `cutoff` - A tupple (n,date) limmiting tries to the n firsts contacts (not supressed or in the past), and not starting after date.
///
/// # Returns
///
/// An (potentially empty) iterator over effectively suitable PathFragment.
fn try_make_hop<'id, 'a, NM: NodeManager + 'a, CM: ContactManager, T: AsRef<Contact<CM>>>(
    graph: &Multigraph<'id, NM, CM>,
    last_hop: (&PathFragment<'id>, usize),
    bundle: &Bundle,
    current_node: INodeRef<'id>,
    contacts: impl Iterator<Item = (RealNodeRef<'id>, &'a NM, ContactRef<'id>, T)>,
    previous_node: Option<INodeRef<'id>>,
    neigbhoor_id: NodeID,
) -> Option<PathFragment<'id>> {
    #[derive(Debug)]
    struct Best<'id> {
        contact: ContactRef<'id>,
        receive: TimeInterval,
        dest: RealNodeRef<'id>,
        send: TimeInterval,
        expiration: Date,
    }

    let send_time = match previous_node {
        None => last_hop.0.recv.end,
        Some(tx_node) => graph[current_node].manager.process_delay(
            bundle,
            last_hop.0.recv,
            tx_node.into(),
            neigbhoor_id,
        ),
    };
    // remove suppressed contacts
    #[allow(unused_variables)]
    let suppressed = contacts.filter(|(_, _, _, ct)| {
        #[cfg(feature = "contact_suppression")]
        if ct.as_ref().suppressed {
            return false;
        }
        true
    });
    let mut best: Option<Best<'id>> = None;

    for (next_node_ref, next_node_manager, ctref, ct) in suppressed {
        if let Some(Best { receive, .. }) = best
            && receive.end <= ct.as_ref().lifespan.start
        {
            break;
        }
        // contact managers
        if let Some(txdata) =
            ct.as_ref()
                .manager
                .dry_run_tx(ct.as_ref().lifespan, send_time, bundle)
        {
            if let Some(Best { receive, .. }) = best
                && receive.end < txdata.recv.end
            {
                continue;
            }
            if !next_node_manager.accept(bundle, txdata.recv, current_node.into()) {
                continue;
            }
            if let Some(previous) = previous_node
                && !graph[current_node].manager.dry_run_retention(
                    bundle,
                    last_hop.0.recv,
                    previous.into(),
                    txdata.send,
                    neigbhoor_id,
                )
            {
                //early return if current node refuse, as it is unlikely making it wait for the bundle longer will make it accept
                //Maybe replace that with the node returning a window of possible send time
                break;
            }
            best = Some(Best {
                contact: ctref,
                receive: txdata.recv,
                dest: next_node_ref,
                send: txdata.send,
                expiration: ct.as_ref().lifespan.end,
            })
        }
    }

    // Former method for exp calcumation (more accurate)
    //
    // route_proposition.cumulative_delay =
    //     sndr_route_borrowed.cumulative_delay + final_data.rx_end - final_data.tx_end;
    // route_proposition.expiration = Date::min(
    //     final_data.expiration - sndr_route_borrowed.cumulative_delay,
    //     sndr_route_borrowed.expiration,
    // );

    best.map(
        |Best {
             contact,
             receive,
             dest,
             send,
             expiration,
         }| PathFragment {
            via: Some(ViaHop {
                contact,
                parent_frag: last_hop.1,
                send,
            }),
            hop_count: last_hop.0.hop_count + 1,
            rx_node: dest,
            recv: receive,
            expiration: expiration.min(last_hop.0.expiration),
        },
    )
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "contact_suppression")]
    use core::error;

    use alloc::vec;

    use super::*;
    use crate::bundle::Bundle;

    use crate::contact_manager::legacy::evl::EVLManager;
    use crate::mk_graph;
    use crate::multigraph::NodeRef;

    use crate::contact_plan::asabr_file_lexer::parse_from_iter;
    use crate::node_manager::NodeManager;
    use crate::node_manager::none::NoManagement;
    use crate::pathfinding::test_helpers::*;
    use generativity::make_guard;

    #[track_caller]
    fn run_hop<'id, 'a, CM: ContactManager + 'id, NM: NodeManager + 'id>(
        graph: &'a Multigraph<'id, NM, CM>,
        bundle: &Bundle,
        current_node: INodeRef<'id>,
        next_node: INodeRef<'id>,
        send_time: Date,
        contacts: impl Iterator<Item = (RealNodeRef<'id>, &'a NM, ContactRef<'id>, &'a Contact<CM>)>,
    ) -> Option<PathFragment<'id>> {
        // Keep the initial fragment alive for the duration of the try_make_hop call to satisfy borrow checker lifetime constraints
        let prev_frag = PathFragment::new_start(send_time, current_node.into());

        try_make_hop(
            graph,
            (&prev_frag, 0),
            bundle,
            current_node,
            contacts,
            None, // No previous node for the initial hop
            next_node.into(),
        )
    }

    fn run_hop_on_graph<A>(
        graph_str: &str,
        bundle: &Bundle,
        f: impl for<'a> FnOnce(Option<PathFragment<'a>>) -> Result<A, ASABRError>,
    ) -> Result<A, ASABRError> {
        let lines = graph_str.lines();
        let contact_plan = parse_from_iter::<NoManagement, EVLManager>(lines).unwrap();
        make_guard!(id);
        let graph = Multigraph::new(id, contact_plan).unwrap();

        let mut refs = Vec::new();
        for i in 0..2 {
            if let Ok(NodeRef::I(re)) = graph.node_id_ref(i.into()) {
                refs.push(re)
            } else {
                panic!("Node {} missing", i)
            }
        }

        // Build the required tuple iterator
        // directly bind the receiver to refs[1] as this simulates a strict A -> B hop
        let contacts_iter = graph
            .iter_contacts(refs[0], refs[1])
            .map(|(c_ref, contact)| {
                let rx_rnode_ref = refs[1];
                let nm = &graph[rx_rnode_ref].manager;

                (rx_rnode_ref.into(), nm, c_ref, contact)
            });

        let r = run_hop::<_, _>(&graph, bundle, refs[0], refs[1], 0, contacts_iter);
        f(r)
    }

    #[test]
    fn test_empty_contacts() {
        let graph = "node 0 A node 1 B";
        // Priority: 1, Size: 100, Expiration: 1000
        let bundle = make_bundle(1, 100, 1000);

        run_hop_on_graph(graph, &bundle, |result| {
            assert!(
                result.is_none(),
                "TEST FAILED: Expected None when contacts iterator is empty."
            );
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_bundle_too_large() {
        let graph = "node 0 A node 1 B
                            contact 0 1 0 200 100 1";

        // Priority: 1, Size: 999_999, Expiration: 1000
        let bundle = make_bundle(1, 999_999, 1000);

        run_hop_on_graph(graph, &bundle, |result| {
            assert!(
                result.is_none(),
                "TEST FAILED: Expected None when the bundle size exceeds contact capacity."
            );
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_single_contact_valid() {
        let graph = "node 0 A node 1 B
                            contact 0 1 0 200 100 1";
        // Priority: 1, Size: 50, Expiration: 1000
        let bundle = make_bundle(1, 50, 1000);

        run_hop_on_graph(graph, &bundle, |result| {
            // A valid contact exists, so try_make_hop should successfully return a PathFragment
            assert!(
                result.is_some(),
                "TEST FAILED: Expected Some when the contact is valid and the bundle size is within contact capacity."
            );
            Ok(())
        }).unwrap();
    }

    #[cfg(feature = "contact_suppression")]
    #[test]
    fn test_all_contacts_suppressed() -> Result<(), alloc::boxed::Box<dyn error::Error>> {
        use generativity::make_guard;

        use crate::contact_plan::asabr_file_lexer::parse_from_iter;

        let graph_str = "node 0 A node 1 B
                            contact 0 1 0 200 100 1
                            contact 0 1 20 100 50 1
                            contact 0 1 10 300 100 1";
        // Setup the graph
        let contact_plan = parse_from_iter::<NoManagement, EVLManager>(graph_str.lines())?;
        make_guard!(id);
        let mut graph = Multigraph::new(id, contact_plan)?;

        let mut refs = Vec::new();

        for i in 0..2 {
            if let Ok(NodeRef::I(re)) = graph.node_id_ref(i.into()) {
                refs.push(re)
            } else {
                panic!("Node {} missing", i)
            }
        }

        // Mutate graph to suppress all contacts
        for (_, ct) in graph.iter_contacts_mut(refs[0], refs[1]) {
            ct.suppressed = true;
        }

        let contacts_iter = graph
            .iter_contacts(refs[0], refs[1])
            .map(|(c_ref, contact)| {
                let rx_rnode_ref = refs[1];
                let nm = &graph[rx_rnode_ref].manager;

                (rx_rnode_ref.into(), nm, c_ref, contact)
            });

        let bundle = make_bundle(1, 100, 1000);

        let result = run_hop::<_, _>(&graph, &bundle, refs[0], refs[1], 0, contacts_iter);

        assert!(
            result.is_none(),
            "TEST FAILED: Expected None when all contacts are suppressed."
        );
        Ok(())
    }

    #[cfg(feature = "contact_suppression")]
    #[test]
    fn test_partial_suppression_uses_valid_contact()
    -> Result<(), alloc::boxed::Box<dyn error::Error>> {
        let graph_str = "node 0 A node 1 B
                            contact 0 1 0 200 100 1
                            contact 0 1 0 200 100 2";
        let contact_plan = parse_from_iter::<NoManagement, EVLManager>(graph_str.lines())?;
        make_guard!(id);
        let mut graph = Multigraph::new(id, contact_plan)?;

        let mut refs = Vec::new();

        for i in 0..2 {
            if let Ok(NodeRef::I(re)) = graph.node_id_ref(i.into()) {
                refs.push(re)
            } else {
                panic!("Node {} missing", i)
            }
        }

        // Suppress ONLY the first contact (the one with delay = 1)
        for (_, ct) in graph.iter_contacts_mut(refs[0], refs[1]).take(1) {
            ct.suppressed = true;
        }

        // Build the updated iterator
        let contacts_iter = graph
            .iter_contacts(refs[0], refs[1])
            .map(|(c_ref, contact)| {
                let rx_rnode_ref = refs[1];
                let nm = &graph[rx_rnode_ref].manager;

                (rx_rnode_ref.into(), nm, c_ref, contact)
            });

        let bundle = make_bundle(1, 100, 1000);

        let result = run_hop::<_, _>(&graph, &bundle, refs[0], refs[1], 0, contacts_iter);

        assert!(
            result.is_some(),
            "TEST FAILED: Expected Some from non-suppressed contact."
        );
        let route = result.unwrap();
        assert_eq!(
            route.recv.end, 3,
            "TEST FAILED: Expected arrival 3 from non-suppressed contact (got {}).",
            route.recv.end
        );
        Ok(())
    }

    #[test]
    fn test_node_tx_refusing() {
        use crate::contact_plan::ContactPlan;
        use generativity::make_guard;

        // Setup the testing elements with MockNodeManager
        let bundle = make_bundle(1, 1, 2000);

        // Create a transmitting node that explicitly refuses to transmit
        let tx_node = make_vertex(0, "A", MockNodeManager::refusing_tx());
        // Create a receiving node that accepts everything
        let rx_node = make_vertex(1, "B", MockNodeManager::accepting());
        let nodes = vec![tx_node, rx_node];

        // tx=0, rx=1, start=0, end=2000, rate=100, delay=1
        let contact_tuple = make_contact(0, 1, 0, 2000, 100, 1);
        let contacts = vec![contact_tuple];

        // Build the ContactPlan and the Graph manually
        let plan = ContactPlan {
            realnodes: nodes,
            vnodes: vec![],
            contacts,
        };

        make_guard!(id);
        let graph = Multigraph::new(id, plan).unwrap();

        // Extract safe references
        let tx_ref = match graph.node_id_ref(0.into()).unwrap() {
            NodeRef::I(r) => r,
            _ => panic!("Expected RNodeRef"),
        };
        let rx_ref = match graph.node_id_ref(1.into()).unwrap() {
            NodeRef::I(r) => r,
            _ => panic!("Expected RNodeRef"),
        };

        // Build the required contact iterator
        // Since we only have 1 contact, we can just grab it by index 0 in the graph
        let contacts_iter = graph.iter_contacts(tx_ref, rx_ref).map(|(c_ref, contact)| {
            let nm = &graph[rx_ref].manager;
            (rx_ref.into(), nm, c_ref, contact)
        });

        // Bypass the run_hop helper to manually inject a previous_node
        let prev_frag = PathFragment::new_start(0, tx_ref.into());
        let result = try_make_hop(
            &graph,
            (&prev_frag, 0),
            &bundle,
            tx_ref,
            contacts_iter,
            Some(tx_ref),
            rx_ref.into(),
        );

        assert!(
            result.is_none(),
            "TEST FAILED: Expected None when tx node refuses to emit."
        );
    }

    #[test]
    fn test_node_rx_refusing() {
        use crate::contact_plan::ContactPlan;
        use generativity::make_guard;

        let bundle = make_bundle(1, 1, 2000);

        let tx_node = make_vertex(0, "A", MockNodeManager::accepting());
        let rx_node = make_vertex(1, "B", MockNodeManager::refusing_rx());
        let nodes = vec![tx_node, rx_node];

        let contact_tuple = make_contact(0, 1, 0, 2000, 100, 1);
        let contacts = vec![contact_tuple];

        let plan = ContactPlan {
            realnodes: nodes,
            vnodes: vec![],
            contacts,
        };
        make_guard!(id);
        let graph = Multigraph::new(id, plan).unwrap();

        let tx_ref = match graph.node_id_ref(0.into()).unwrap() {
            NodeRef::I(r) => r,
            _ => panic!("Expected RNodeRef"),
        };
        let rx_ref = match graph.node_id_ref(1.into()).unwrap() {
            NodeRef::I(r) => r,
            _ => panic!("Expected RNodeRef"),
        };

        let contacts_iter = graph.iter_contacts(tx_ref, rx_ref).map(|(c_ref, contact)| {
            let nm = &graph[rx_ref].manager;
            (rx_ref.into(), nm, c_ref, contact)
        });

        let result = run_hop::<_, _>(&graph, &bundle, tx_ref, rx_ref, 0, contacts_iter);

        assert!(
            result.is_none(),
            "TEST FAILED: Expected None when rx node refuses to receive."
        );
    }

    #[test]
    fn test_node_proc_delay() {
        use crate::contact_plan::ContactPlan;
        use generativity::make_guard;

        // Setup bundle: Priority 1, Size 100 (integer division), Expiration 2000
        let bundle = make_bundle(1, 100, 2000);

        let tx_node = make_vertex(0, "A", MockNodeManager::processing(2));
        let rx_node = make_vertex(1, "B", MockNodeManager::accepting());
        let nodes = vec![tx_node, rx_node];

        let contact_tuple = make_contact(0, 1, 0, 2000, 100, 1);
        let contacts = vec![contact_tuple];

        let plan = ContactPlan {
            realnodes: nodes,
            vnodes: vec![],
            contacts,
        };

        make_guard!(id);
        let graph = Multigraph::new(id, plan).unwrap();

        let tx_ref = match graph.node_id_ref(0.into()).unwrap() {
            NodeRef::I(r) => r,
            _ => panic!("Expected RNodeRef"),
        };
        let rx_ref = match graph.node_id_ref(1.into()).unwrap() {
            NodeRef::I(r) => r,
            _ => panic!("Expected RNodeRef"),
        };

        let contacts_iter = graph.iter_contacts(tx_ref, rx_ref).map(|(c_ref, contact)| {
            let nm = &graph[rx_ref].manager;
            (rx_ref.into(), nm, c_ref, contact)
        });

        // Must simulate a relay scenario so the processing delay is taken into account
        let prev_frag = PathFragment::new_start(0, tx_ref.into());

        let result = try_make_hop(
            &graph,
            (&prev_frag, 0),
            &bundle,
            tx_ref,
            contacts_iter,
            Some(tx_ref), // Simulate relay to trigger processing delay
            rx_ref.into(),
        );

        assert!(
            result.is_some(),
            "TEST FAILED: Expected Some even with node processing delay."
        );
        let route = result.unwrap();

        // Expected arrival:
        // Base time (0) + Proc Delay (2) = Send Time (2)
        // Tx Duration: Size(100) / Rate(100) = 1
        // Contact Delay = 1
        // Arrival = Send Time(2) + Duration(1) + Delay(1) = 4
        assert_eq!(
            route.recv.end, 4,
            "TEST FAILED: Arrival should account for the 2s node processing delay (expected 4, got {}).",
            route.recv.end
        );
    }

    #[test]
    fn test_best_contact_selected_1_hop() -> Result<(), ASABRError> {
        let graph_str = "node 0 A node 1 B
                            contact 0 1 0 50 100 5
                            contact 0 1 0 200 100 2
                            contact 0 1 10 100 50 1
                            contact 0 1 20 30 100 1";
        let bundle = make_bundle(1, 100, 2000);

        mk_graph!(graph, NoManagement, EVLManager, graph_str, raw);

        let mut refs = Vec::new();
        for i in 0..2 {
            if let Ok(NodeRef::I(re)) = graph.node_id_ref(i.into()) {
                refs.push(re)
            } else {
                panic!("Node {} missing", i)
            }
        }

        let contacts_iter = graph
            .iter_contacts(refs[0], refs[1])
            .map(|(c_ref, contact)| {
                let rx_rnode_ref = refs[1];
                let nm = &graph[rx_rnode_ref].manager;
                (rx_rnode_ref.into(), nm, c_ref, contact)
            });

        // run the hop, simulating that the bundle is ready to be sent at t=5
        let result = run_hop::<_, _>(
            &graph,
            &bundle,
            refs[0],
            refs[1],
            5, // The bundle is available at Node A at t=5
            contacts_iter,
        );
        assert!(
            result.is_some(),
            "TEST FAILED: Expected Some, at least one contact should be valid."
        );
        let route = result.unwrap();

        // Contact 2 should have been selected
        // Send time = max(bundle_arrival=5, contact_start=0) = 5
        // Tx Duration = size(100) / rate(100) = 1
        // Delay = 2
        // Arrival = 5 + 1 + 2 = 8
        assert_eq!(
            route.recv.end, 8,
            "TEST FAILED: Expected arrival 8 from the optimal contact (got {}).",
            route.recv.end
        );
        assert_eq!(
            route.hop_count, 1,
            "TEST FAILED: Expected hop_count = 1 (got {}).",
            route.hop_count
        );
        assert!(
            route.via.is_some(),
            "TEST FAILED: Expected a ViaHop to be set."
        );
        Ok(())
    }

    #[test]
    fn test_best_contact_selected_2_hops() {
        // Setup a 3-node graph using the string format
        // Node 0 -> Node 1 (Hop 1)
        // Contact A (delay 1) and Contact B (delay 5)
        // Node 1 -> Node 2 (Hop 2)
        // Contact C (delay 1, converted from 0.5) and Contact D (delay 2)
        let graph_str = "node 0 A node 1 B node 2 C
                            contact 0 1 0 200 100 1
                            contact 0 1 0 200 100 5
                            contact 1 2 0 1000 100 1
                            contact 1 2 0 1000 100 2";

        let bundle = make_bundle(1, 100, 2000);

        let lines = graph_str.lines();
        let contact_plan = parse_from_iter::<NoManagement, EVLManager>(lines).unwrap();
        make_guard!(id);
        let graph = Multigraph::new(id, contact_plan).unwrap();

        let mut refs = Vec::new();
        // Extract Node 0, Node 1, and Node 2
        for i in 0..3 {
            if let Ok(NodeRef::I(re)) = graph.node_id_ref(i.into()) {
                refs.push(re)
            } else {
                panic!("Node {} missing", i)
            }
        }

        // hop 1
        let contacts_iter_1 = graph
            .iter_contacts(refs[0], refs[1])
            .map(|(c_ref, contact)| {
                let nm = &graph[refs[1]].manager;
                (refs[1].into(), nm, c_ref, contact)
            });

        // Hop 1 originates at Node 0 at time 0
        let hop1 = run_hop::<_, _>(&graph, &bundle, refs[0], refs[1], 0, contacts_iter_1)
            .expect("TEST FAILED: Hop 1 should succeed.");

        // Verification hop 1
        // Tx Duration = 100 / 100 = 1. Delay = 1 (Contact A). Arrival = 2.
        assert_eq!(
            hop1.recv.end, 2,
            "Hop 1 FAILED: Expected arrival 2 (got {}).",
            hop1.recv.end
        );
        assert_eq!(
            hop1.hop_count, 1,
            "Hop 1 FAILED: Expected hop_count = 1 (got {}).",
            hop1.hop_count
        );

        // hop 2
        let contacts_iter_2 = graph
            .iter_contacts(refs[1], refs[2])
            .map(|(c_ref, contact)| {
                let nm = &graph[refs[2]].manager;
                (refs[2].into(), nm, c_ref, contact)
            });

        // We call try_make_hop directly to pass hop1 as the previous fragment
        // It acts as a relay from Node 1 to Node 2.
        let hop2 = try_make_hop(
            &graph,
            (&hop1, 0), // Pass the previous fragment and its theoretical index
            &bundle,
            refs[1], // Current node is Node 1
            contacts_iter_2,
            Some(refs[0]), // Previous node was Node 0
            refs[2].into(),
        )
        .expect("TEST FAILED: Hop 2 should succeed.");

        // Verification Hop 2
        // Start time = Hop 1 Arrival = 2
        // Tx Duration = 100 / 100 = 1. Delay = 1 (Contact C). Arrival = 2 + 1 + 1 = 4.
        assert_eq!(
            hop2.recv.end, 4,
            "Hop 2 FAILED: Expected arrival 4 (got {}).",
            hop2.recv.end
        );
        assert_eq!(
            hop2.hop_count, 2,
            "Hop 2 FAILED: Expected hop_count = 2 (got {}).",
            hop2.hop_count
        );
        assert!(
            hop2.via.is_some(),
            "Hop 2 FAILED: Expected a ViaHop to be set."
        );
    }

    #[test]
    fn test_to_node_equals_receiver_vertex_id() {
        let graph_str = "node 0 A node 1 B
                            contact 0 1 0 200 100 1";

        let bundle = make_bundle(1, 50, 1000);

        let lines = graph_str.lines();
        let contact_plan = parse_from_iter::<NoManagement, EVLManager>(lines).unwrap();
        make_guard!(id);
        let graph = Multigraph::new(id, contact_plan).unwrap();

        let mut refs = Vec::new();
        for i in 0..2 {
            if let Ok(NodeRef::I(re)) = graph.node_id_ref(i.into()) {
                refs.push(re)
            } else {
                panic!("Node {} missing", i)
            }
        }

        let contacts_iter = graph
            .iter_contacts(refs[0], refs[1])
            .map(|(c_ref, contact)| {
                let rx_rnode_ref = refs[1];
                let nm = &graph[rx_rnode_ref].manager;
                (rx_rnode_ref.into(), nm, c_ref, contact)
            });

        let result = run_hop::<_, _>(&graph, &bundle, refs[0], refs[1], 0, contacts_iter);

        let route = result.expect("Expected a valid hop");

        // verify that it matches our exact receiver node reference
        assert_eq!(
            route.rx_node,
            refs[1].into(),
            "rx_node should exactly match the receiver RNodeRef"
        );

        // If we strictly want to check the ID as an integer just to be absolutely sure
        let rx_id: usize = route.rx_node.internal().unwrap().into();
        assert_eq!(rx_id, 1, "rx_node ID should be 1, got {}", rx_id);
    }
}
