extern crate alloc;
use alloc::vec::Vec;
use core::fmt::Debug;
use core::ops::Deref;
use generativity::Guard;

use crate::bundle::Bundle;
use crate::contact::Contact;
use crate::contact_manager::ContactManager;
use crate::errors::ASABRError;
use crate::multigraph::{ContactRef, Multigraph, NodeRef, RNodeRef};
use crate::node_manager::NodeManager;
use crate::parsing::Either;
use crate::paths::{PathFragment, ViaHop};
use crate::types::{Date, NodeID, TimeInterval};

#[cfg(feature = "contact_work_area")]
pub mod contact_parenting;
pub mod hybrid_parenting;
#[cfg(feature = "contact_suppression")]
pub mod limiting_contact;
pub mod node_parenting;
#[cfg(test)]
mod test_helpers;

/// Data structure that holds the results of a pathfinding operation.
///
/// This struct encapsulates information necessary for the outcome of a pathfinding algorithm,
/// including the associated bundle, excluded nodes, and organized route stages by destination.
///
/// # Type Parameters
///
/// * `NM` - A generic type that implements the `NodeManager` trait.
/// * `CM` - A generic type that implements the `ContactManager` trait.
#[derive(Debug)]
pub struct PathFindingOutput<'id, 'a> {
    pub path_tree: Either<&'a [Option<PathFragment<'id>>], Vec<Option<PathFragment<'id>>>>,
}

impl<'id, 'a> AsRef<[Option<PathFragment<'id>>]> for PathFindingOutput<'id, 'a> {
    fn as_ref(&self) -> &[Option<PathFragment<'id>>] {
        match &self.path_tree {
            Either::Left(l) => *l,
            Either::Right(r) => r.as_ref(),
        }
    }
}

impl<'id, 'a> Deref for PathFindingOutput<'id, 'a> {
    type Target = [Option<PathFragment<'id>>];
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<'id, 'a> PathFindingOutput<'id, 'a> {
    /// Return the list of hops making this path, if it is still a valid (and detected) one,
    pub fn get_full_path<NM: NodeManager, CM: ContactManager>(
        &self,
        destination: RNodeRef<'id>,
    ) -> Option<Vec<PathFragment<'id>>> {
        let mut next = self[NodeID::from(destination) as usize]?;
        let mut r = Vec::with_capacity(next.hop_count as usize + 1);
        r.push(next);
        while let Some(next_via) = next.via {
            next = self[next_via.parent_frag]?;
            r.push(next);
        }
        Some(r)
    }
}

/// The `Pathfinding` trait provides the interface for implementing a pathfinding algorithm.
/// It requires methods for creating a new instance and determining the next hop in a route.
///
/// # Type Parameters
///
/// * `NM` - A generic type that implements the `NodeManager` trait.
/// * `CM` - A generic type that implements the `ContactManager` trait.
pub trait Pathfinding<'id, NM: NodeManager, CM: ContactManager> {
    /// Creates a new instance of the pathfinding algorithm with the provided nodes and contacts.
    ///
    /// # Parameters
    ///
    /// * `multigraph` - A reference-counted, mutable pointer to the multigraph containing nodes and contacts for pathfinding.
    ///
    /// # Returns
    ///
    /// A new instance of the struct implementing `Pathfinding`.
    fn new(id: Guard, multigraph: &Multigraph<'id, NM, CM>) -> Self;

    /// Determines the next hop in the route for the given bundle, excluding specified nodes.
    ///
    /// # Parameters
    ///
    /// * `current_time` - The current time for the pathfinding operation.
    /// * `source` - The `NodeID` of the source node.
    /// * `bundle` - A reference to the `Bundle` being routed.
    /// * `excluded_nodes_sorted` - A vector of `NodeID`s that should be excluded from the pathfinding.
    ///
    /// # Returns
    ///
    /// A `Result<PathFindingOutput<NM, CM>, ASABRError>` containing the results of the pathfinding operation,
    /// or an error if the operation fails.
    fn find_path<'a>(
        &'a mut self,
        multigraph: &mut Multigraph<'id, NM, CM>,
        current_time: Date,
        source: NodeRef<'id>,
        bundle: &Bundle,
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
fn try_make_hop<'id, NM: NodeManager, CM: ContactManager, T: AsRef<Contact<CM>>>(
    graph: &Multigraph<'id, NM, CM>,
    last_hop: (&PathFragment<'id>, usize),
    bundle: &Bundle,
    current_node: RNodeRef<'id>,
    next_node: RNodeRef<'id>,
    send_time: Date,
    contacts: impl Iterator<Item = (ContactRef<'id>, T)>,
) -> Option<PathFragment<'id>> {
    // remove suppressed contacts
    let suppressed = contacts.filter(|(_, ct)| {
        #[cfg(feature = "contact_suppression")]
        if ct.as_ref().suppressed {
            return false;
        }
        true
    });
    let mut best: Option<(ContactRef<'id>, TimeInterval)> = None;

    for (ctref, ct) in suppressed {
        // not better
        if let Some((_, time)) = best
            && time.end <= ct.as_ref().lifespan.start
        {
            break;
        }
        // contact managers
        if let Some(txdata) =
            ct.as_ref()
                .manager
                .dry_run_tx(ct.as_ref().lifespan, send_time, bundle)
        {
            if let Some((_, time)) = best
                && time.end < txdata.rx_window.end
            {
                continue;
            }
            if !graph[next_node]
                .manager
                .accept(bundle, txdata.rx_window, current_node.into())
            {
                continue;
            }
            if let Some(previous) = &last_hop.0.via
                && !graph[current_node].manager.dry_run_retention(
                    bundle,
                    last_hop.0.arrival_time,
                    previous.tx_node.into(),
                    txdata.tx_window,
                    next_node.into(),
                )
            {
                //early return if current node refuse, as it is unlikely making it wait for the bundle longer will make it accept
                //Maybe replace that with the node returning a window of possible send time
                break;
            }
            best = Some((ctref, txdata.rx_window))
        }
    }

    best.map(|(ct_ref, time)| PathFragment {
        via: Some(ViaHop {
            contact: ct_ref,
            parent_frag: last_hop.1,
            tx_node: current_node,
        }),
        hop_count: last_hop.0.hop_count + 1,
        arrival_time: time,
    })
}

// #[cfg(test)]
// mod tests {
//     #[cfg(feature = "contact_suppression")]
//     use core::error;

//     use super::*;
//     use crate::bundle::Bundle;

//     use crate::contact_manager::legacy::evl::EVLManager;
//     use crate::multigraph::NodeRef;

//     use crate::node_manager::NodeManager;
//     use crate::node_manager::none::NoManagement;
//     use crate::pathfinding::test_helpers::*;
//     use crate::{distance, mk_graph_pathfinding, pathfinding};

//     #[track_caller]
//     fn run_hop<'id, T: Pathfinding<'id, NM, CM>, CM: ContactManager, NM: NodeManager>(
//         graph: &Multigraph<'id, NM, CM>,
//         bundle: &Bundle,
//         current_node: RNodeRef<'id>,
//         next_node: RNodeRef<'id>,
//         send_time: Date,
//         contacts: impl Iterator<Item = ContactRef<'id>>,
//     ) -> Option<PathFragment<'id>> {
//         try_make_hop(
//             graph,
//             (
//                 &PathFragment {
//                     via: None,
//                     hop_count: 0,
//                     arrival_time: TimeInterval { start: 0, end: 0 },
//                 },
//                 0,
//             ),
//             bundle,
//             current_node,
//             next_node,
//             send_time,
//             contacts,
//         )
//     }

//     type Finder<'id> = pathfinding::hybrid_parenting::HybridParenting<
//         'id,
//         false,
//         NoManagement,
//         EVLManager,
//         distance::sabr::SABR,
//     >;

//     fn run_hop_on_graph<A>(
//         graph: &str,
//         bundle: &Bundle,
//         f: impl for<'a> FnOnce(Option<PathFragment<'a>>) -> Result<A, ASABRError>,
//     ) -> Result<A, ASABRError> {
//         mk_graph_pathfinding!(graph, finder, NoManagement, EVLManager, Finder, graph, raw);
//         let mut refs = Vec::new();
//         for i in 0..1 {
//             if let Ok(NodeRef::R(re)) = graph.node_id_ref(i) {
//                 refs.push(re)
//             } else {
//                 panic!("")
//             }
//         }
//         let r = run_hop(
//             &mut graph,
//             bundle,
//             refs[0],
//             refs[1],
//             0.0,
//             graph.iter_contacts(refs[0], refs[1]),
//         );
//         f(r)
//     }

//     #[test]
//     fn test_empty_contacts() {
//         // let bundle = make_bundle(1, 1, 50.0, 2000.0);
//         // let source = make_source::<NoManagement>(0.0, 0, &bundle);
//         let graph = "node 0 A node 1 B";
//         let bundle = make_bundle(1, 1, 100.0, 1000.0);
//         run_hop_on_graph(graph, &bundle, |result| {
//             assert!(
//                 result.is_none(),
//                 "TEST FAILED: Expected None when contacts iterator is empty."
//             );
//             Ok(())
//         });
//     }

//     #[test]
//     fn test_bundle_too_large() {
//         let graph = "node 0 A node 1 B
//                             contact 0 1 0 200 100 1";
//         run_hop_on_graph(graph, &make_bundle(1, 1, 999_999., 1000.), |result| {
//             assert!(
//                 result.is_none(),
//                 "TEST FAILED: Expected None when the bundle size exceeds contact capacity."
//             );
//             Ok(())
//         });
//     }

//     #[test]
//     fn test_single_contact_valid() {
//         let graph = "node 0 A node 1 B
//                             contact 0 1 0 200 100 1";
//         run_hop_on_graph(graph, &make_bundle(1, 1, 50., 1000.), |result| {
//             assert!(
//                 result.is_some(),
//                 "TEST FAILED: Expected Some when the contact is valid and the bundle size is within contact capacity."
//             );
//             Ok(())
//         });
//     }

//     #[cfg(feature = "contact_suppression")]
//     #[test]
//     fn test_all_contacts_suppressed() -> Result<(), alloc::boxed::Box<dyn error::Error>> {
//         use generativity::make_guard;

//         use crate::contact_plan::asabr_file_lexer::parse_from_iter;

//         let graph = "node 0 A node 1 B
//                             contact 0 1 0 200 100 1
//                             contact 0 1 20 100 50 1
//                             contact 0 1 10 300 100 1"
//             .lines();
//         make_guard!(id);
//         let mut graph =
//             Multigraph::<'_, NoManagement, EVLManager>::new(id, parse_from_iter(graph)?)?;

//         let mut refs = Vec::new();

//         for i in 0..1 {
//             if let Ok(NodeRef::R(re)) = graph.node_id_ref(i) {
//                 refs.push(re)
//             } else {
//                 panic!("")
//             }
//         }
//         for (_, ct) in graph.iter_contacts_mut(refs[0], refs[1]) {
//             ct.suppressed = true
//         }
//         let result = run_hop(
//             &mut graph,
//             &make_bundle(1, 1, 100., 1000.),
//             refs[0],
//             refs[1],
//             0.0,
//             graph.iter_contacts(refs[0], refs[1]),
//         );

//         assert!(
//             result.is_none(),
//             "TEST FAILED: Expected None when all contacts are suppressed."
//         );
//         Ok(())
//     }

//     #[cfg(feature = "contact_suppression")]
//     #[test]
//     fn test_partial_suppression_uses_valid_contact()
//     -> Result<(), alloc::boxed::Box<dyn error::Error>> {
//         use generativity::make_guard;

//         use crate::contact_plan::asabr_file_lexer::parse_from_iter;

//         let graph = "node 0 A node 1 B
//                             contact 0 1 0 200 100 1
//                             contact 0 1 0 200 100 2"
//             .lines();
//         make_guard!(id);
//         let mut graph =
//             Multigraph::<'_, NoManagement, EVLManager>::new(id, parse_from_iter(graph)?)?;

//         let mut refs = Vec::new();

//         for i in 0..1 {
//             if let Ok(NodeRef::R(re)) = graph.node_id_ref(i) {
//                 refs.push(re)
//             } else {
//                 panic!("")
//             }
//         }
//         for (_, ct) in graph.iter_contacts_mut(refs[0], refs[1]).take(1) {
//             ct.suppressed = true
//         }
//         let result = run_hop(
//             &mut graph,
//             &make_bundle(1, 1, 100., 1000.),
//             refs[0],
//             refs[1],
//             0.0,
//             graph.iter_contacts(refs[0], refs[1]),
//         );

//         assert!(
//             result.is_some(),
//             "TEST FAILED: Expected Some from non-suppressed contact."
//         );
//         let route = result.unwrap();
//         assert_eq!(
//             route.arrival_time.end, 2.1,
//             "TEST FAILED: Expected arrival 2.1 from non-suppressed contact (got {}).",
//             route.arrival_time.end
//         );
//         Ok(())
//     }

//     #[test]
//     fn test_node_tx_refusing() {
//         use generativity::make_guard;

// use crate::contact_plan::ContactPlan;

//         let bundle = make_bundle(1, 1, 1.0, 2000.0);
//         let source = make_source::<MockNodeManager>(0.0, 0, &bundle);
//         let tx = make_vertex(0, "A", MockNodeManager::refusing_tx());
//         let rx = make_vertex(1, "B", MockNodeManager::accepting());
//         let nodes = vec![tx, rx];
//         let contacts = vec![make_contact::<MockNodeManager>(
//             0, 1, 0.0, 2000.0, 100.0, 1.0,
//         )];

//         make_guard!(id);
//         let graph = Multigraph::new(id, ContactPlan{
//             realnodes: nodes,
//             vnodes: vec![],
//             contacts: contacts,
//         });

//         let result = try_make_hop(&graph?,  todo!(),&bundle, );

//         assert!(
//             result.is_none(),
//             "TEST FAILED: Expected None when tx node refuses to emit."
//         );
//     }

//     #[cfg(feature = "node_rx")]
//     #[test]
//     fn test_node_rx_refusing() {
//         let bundle = make_bundle(1, 1, 1.0, 2000.0);
//         let source = make_source::<MockNodeManager>(0.0, 0, &bundle);
//         let tx = make_node_rc(0, "A", MockNodeManager::accepting());
//         let rx = make_node_rc(1, "B", MockNodeManager::refusing_rx());
//         let nodes = vec![tx, rx];
//         let contacts = vec![make_contact_rc::<MockNodeManager>(
//             0, 1, 0.0, 2000.0, 100.0, 1.0,
//         )];

//         let result = run_hop(0, &source, &bundle, 1, &contacts, &nodes);

//         assert!(
//             result.is_none(),
//             "TEST FAILED: Expected None when rx node refuses to receive."
//         );
//     }

//     #[cfg(feature = "node_proc")]
//     #[test]
//     fn test_node_proc_delay() {
//         let bundle = make_bundle(1, 1, 10.0, 2000.0);
//         let source = make_source::<MockNodeManager>(0.0, 0, &bundle);
//         let tx = make_node_rc(0, "A", MockNodeManager::processing(2.0));
//         let rx = make_node_rc(1, "B", MockNodeManager::accepting());
//         let nodes = vec![tx, rx];
//         let contacts = vec![make_contact_rc::<MockNodeManager>(
//             0, 1, 0.0, 2000.0, 100.0, 1.0,
//         )];

//         let result = run_hop(0, &source, &bundle, 1, &contacts, &nodes);

//         assert!(
//             result.is_some(),
//             "TEST FAILED: Expected Some even with node processing delay."
//         );
//         let route = result.unwrap();
//         // without node_proc : sending_time = 0.0 -> tx_end = 0.1 -> arrival = 1.1
//         // with node_proc : sending_time = 2.0 -> tx_end = 2.1 -> arrival = 3.1
//         assert_eq!(
//             route.at_time, 3.1,
//             "TEST FAILED: Arrival should account for the 2s node processing delay (expected 3.1, got {}).",
//             route.at_time
//         );
//     }

//     #[test]
//     fn test_best_contact_selected_1_hop() {
//         let bundle = make_bundle(1, 1, 100.0, 2000.0);
//         let source = make_source::<NoManagement>(5.0, 0, &bundle);
//         let tx = make_node_rc(0, "A", NoManagement {});
//         let rx = make_node_rc(1, "B", NoManagement {});
//         let nodes = vec![tx, rx];
//         // Contact A : arrival = 11.0
//         let contact_a = make_contact_rc::<NoManagement>(0, 1, 0.0, 50.0, 100.0, 5.0);
//         // Contact B : arrival = 8.0 -> should be the one returned
//         let contact_b = make_contact_rc::<NoManagement>(0, 1, 0.0, 200.0, 100.0, 2.0);
//         // Contact C : start = 10.0 > arrival(8.0) -> pruned
//         let contact_c = make_contact_rc::<NoManagement>(0, 1, 10.0, 100.0, 50.0, 1.0);
//         // Contact D : start = 20.0 > arrival(8.0) -> pruned
//         let contact_d = make_contact_rc::<NoManagement>(0, 1, 20.0, 30.0, 100.0, 0.5);

//         let result = run_hop(
//             0,
//             &source,
//             &bundle,
//             1,
//             &[contact_a, contact_b, contact_c, contact_d],
//             &nodes,
//         );

//         assert!(
//             result.is_some(),
//             "TEST FAILED: Expected Some, at least one contact should be valid."
//         );
//         let route = result.unwrap();

//         // Contact B should have been selected : arrival = tx_end(6.0) + delay(2.0) = 8.0
//         assert_eq!(
//             route.at_time, 8.0,
//             "TEST FAILED: Expected arrival 8.0 from contact B (got {}).",
//             route.at_time
//         );
//         assert_eq!(
//             route.hop_count, 1,
//             "TEST FAILED: Expected hop_count = 1 (got {}).",
//             route.hop_count
//         );
//         assert_eq!(
//             route.cumulative_delay, 2.0,
//             "TEST FAILED: Expected cumulative_delay=2.0 from contact B delay (got {}).",
//             route.cumulative_delay
//         );
//         assert_eq!(
//             route.expiration, 200.0,
//             "TEST FAILED: Expected expiration = 200.0 from contact B end (got {}).",
//             route.expiration
//         );
//         assert!(
//             route.via.is_some(),
//             "TEST FAILED: Expected a ViaHop to be set."
//         );
//     }

//     #[test]
//     fn test_best_contact_selected_2_hops() {
//         let ctx = make_hop_context(100.0);
//         // We set the expiration on the source to test that min(contact.end - cumulative_delay, source.expiration) works
//         ctx.source.borrow_mut().expiration = 150.0;

//         // Contact A : arrival = 2.0 -> the best one
//         let contact_a = make_contact_rc::<NoManagement>(0, 1, 0.0, 200.0, 100.0, 1.0);
//         // Contact B : arrival = 6.0
//         let contact_b = make_contact_rc::<NoManagement>(0, 1, 0.0, 200.0, 100.0, 5.0);

//         let hop1 = run_hop(
//             0,
//             &ctx.source,
//             &ctx.bundle,
//             1,
//             &[contact_a, contact_b],
//             &ctx.nodes,
//         )
//         .expect("TEST FAILED: Hop 1 should succeed.");

//         assert_eq!(
//             hop1.at_time, 2.0,
//             "Hop 1 FAILED: Expected arrival 2.0 (got {}).",
//             hop1.at_time
//         );
//         assert_eq!(
//             hop1.hop_count, 1,
//             "Hop 1 FAILED: Expected hop_count = 1 (got {}).",
//             hop1.hop_count
//         );
//         assert_eq!(
//             hop1.cumulative_delay, 1.0,
//             "Hop 1 FAILED: Expected cumulative_delay = 1.0 (got {}).",
//             hop1.cumulative_delay
//         );
//         // min(contact_a.end(200.0) - cumulative_delay(0.0), source.expiration(150.0)) = 150.0
//         assert_eq!(
//             hop1.expiration, 150.0,
//             "Hop 1 FAILED: Expected expiration = 150.0 limited by source.expiration (got {}).",
//             hop1.expiration
//         );

//         // We take the result of the first hop as a new source
//         let source2: SharedRouteStage<NoManagement, EVLManager> = Rc::new(RefCell::new(hop1));
//         let tx1 = make_node_rc(1, "B", NoManagement {});
//         let rx2 = make_node_rc(2, "C", NoManagement {});
//         let node0 = &ctx.nodes[0]; // Copy the first node previously built, so we have the complete
//         // 3-node graph.
//         let nodes = vec![node0.clone(), tx1, rx2];

//         // Contacts with end = 1000.0 so that source2.expiration is the limiting factor
//         // Contact C : arrival = 3.5 -> the best one
//         let contact_c = make_contact_rc::<NoManagement>(1, 2, 0.0, 1000.0, 100.0, 0.5);
//         // Contact D : arrival = 5.0
//         let contact_d = make_contact_rc::<NoManagement>(1, 2, 0.0, 1000.0, 100.0, 2.0);

//         let hop2 = run_hop(0, &source2, &ctx.bundle, 2, &[contact_c, contact_d], &nodes)
//             .expect("TEST FAILED: Hop 2 should succeed.");

//         assert_eq!(
//             hop2.at_time, 3.5,
//             "Hop 2 FAILED: Expected arrival 3.5 (got {}).",
//             hop2.at_time
//         );
//         assert_eq!(
//             hop2.hop_count, 2,
//             "Hop 2 FAILED: Expected hop_count=2 (got {}).",
//             hop2.hop_count
//         );
//         assert_eq!(
//             hop2.cumulative_delay, 1.5,
//             "Hop 2 FAILED: Expected cumulative_delay=1.5 (got {}).",
//             hop2.cumulative_delay
//         );
//         // min(contact_c.end(1000.0) - cumulative_delay(1.0), source2.expiration(150.0)) = 150.0
//         assert_eq!(
//             hop2.expiration, 150.0,
//             "Hop 2 FAILED: Expected expiration=150.0 limited by propagated source.expiration (got {}).",
//             hop2.expiration
//         );
//         assert!(
//             hop2.via.is_some(),
//             "Hop 2 FAILED: Expected a ViaHop to be set."
//         );
//     }

//     #[test]
//     fn test_to_node_equals_receiver_vertex_id() {
//         let ctx = make_hop_context(50.0);
//         let contacts = vec![make_contact_rc::<NoManagement>(
//             0, 1, 0.0, 200.0, 100.0, 1.0,
//         )];

//         // Pass receiver_id = 1 (same as contact's rx_node_id): to_node should be 1
//         let result = run_hop(0, &ctx.source, &ctx.bundle, 1, &contacts, &ctx.nodes);
//         let route = result.expect("Expected a valid hop");
//         assert_eq!(
//             route.to_node, 1,
//             "to_node should equal the receiver_vertex_id (1), got {}",
//             route.to_node
//         );
//     }

//     #[test]
//     fn test_vnode_receiver_sets_to_node() {
//         let ctx = make_hop_context(50.0);
//         // Contact goes from real node 0 to real node 1
//         let contacts = vec![make_contact::<NoManagement>(0, 1, 0.0, 200.0, 100.0, 1.0)];

//         // Pass receiver_id = 42 (a vnode vertex ID, distinct from contact's rx_node_id = 1).
//         // to_node must be set to the receiver vertex ID, not the contact's rx_node_id.
//         let result = run_hop(0, &ctx.source, &ctx.bundle, 42, &contacts, &ctx.nodes);
//         let route = result.expect("Expected a valid hop even with a vnode receiver");
//         assert_eq!(
//             route.to_node, 42,
//             "to_node should equal the vnode receiver_vertex_id (42), got {}",
//             route.to_node
//         );

//         // The ViaHop should still reference the real nodes from the contact
//         let via = route.via.as_ref().expect("Expected a ViaHop");
//         assert_eq!(
//             via.tx_node.borrow().info.id,
//             0,
//             "ViaHop tx_node should be the real tx node (0)"
//         );
//         assert_eq!(
//             via.rx_node.borrow().info.id,
//             1,
//             "ViaHop rx_node should be the real rx node (1), not the vnode"
//         );
//     }
// }
