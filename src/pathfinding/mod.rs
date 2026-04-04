use crate::bundle::Bundle;
use crate::contact::Contact;
use crate::contact_manager::{ContactManager, ContactManagerTxData};
use crate::errors::ASABRError;
use crate::multigraph::Multigraph;
use crate::node::Node;
use crate::node_manager::NodeManager;
use crate::route_stage::ViaHop;
use crate::route_stage::{RouteStage, SharedRouteStage};
use crate::types::{Date, NodeID};
use std::cell::RefCell;
use std::rc::Rc;

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
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct PathFindingOutput<NM: NodeManager, CM: ContactManager> {
    /// The `Bundle` for which the pathfinding is being performed.
    pub bundle: Bundle,
    /// The `source` RouteStage from which the pathfinding is being performed.
    pub source: SharedRouteStage<NM, CM>,
    /// A list of `NodeID`s representing nodes that should be excluded from the pathfinding.
    pub excluded_nodes_sorted: Vec<NodeID>,
    /// A vector that contains a `RouteStage`s for a specific destination node ID as the index.
    pub by_destination: Vec<Option<SharedRouteStage<NM, CM>>>,
}

pub type SharedPathFindingOutput<NM, CM> = Rc<RefCell<PathFindingOutput<NM, CM>>>;

impl<NM: NodeManager, CM: ContactManager> PathFindingOutput<NM, CM> {
    /// Creates a new `PathFindingOutput` instance, initializing the `by_destination` vector
    /// with empty vectors for each destination node and sorting the excluded nodes.
    ///
    /// # Parameters
    ///
    /// * `bundle` - A reference to the `Bundle` that is part of the pathfinding operation.
    /// * `source` - The source RouteStage from which the pathfinding is being performed.
    /// * `excluded_nodes_sorted` - A vector of `NodeID`s representing nodes to be excluded.
    /// * `node_count` - The total number of nodes in the graph.
    ///
    /// # Returns
    ///
    /// A new `PathFindingOutput` instance.
    pub fn new(
        bundle: &Bundle,
        source: SharedRouteStage<NM, CM>,
        excluded_nodes_sorted: &[NodeID],
        node_count: usize,
    ) -> Self {
        let exclusions = excluded_nodes_sorted.to_vec();
        Self {
            bundle: bundle.clone(),
            source,
            excluded_nodes_sorted: exclusions,
            by_destination: vec![None; node_count],
        }
    }

    pub fn get_source_route(&self) -> SharedRouteStage<NM, CM> {
        self.source.clone()
    }

    /// Initializes the route for a given destination in the routing stage.
    ///
    /// Dijkstra finds the reverse path, this method set up the path.
    ///
    /// # Parameters
    ///
    /// * `destination` - The target node ID for the routing.
    pub fn init_for_destination(&self, destination: NodeID) -> Result<(), ASABRError> {
        if let Some(route) = self.by_destination[destination as usize].clone() {
            RouteStage::init_route(route)?;
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
pub trait Pathfinding<NM: NodeManager, CM: ContactManager> {
    /// Creates a new instance of the pathfinding algorithm with the provided nodes and contacts.
    ///
    /// # Parameters
    ///
    /// * `multigraph` - A reference-counted, mutable pointer to the multigraph containing nodes and contacts for pathfinding.
    ///
    /// # Returns
    ///
    /// A new instance of the struct implementing `Pathfinding`.
    fn new(multigraph: Rc<RefCell<Multigraph<NM, CM>>>) -> Self;

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
    fn get_next(
        &mut self,
        current_time: Date,
        source: NodeID,
        bundle: &Bundle,
        excluded_nodes_sorted: &[NodeID],
    ) -> Result<PathFindingOutput<NM, CM>, ASABRError>;

    /// Get a shared pointer to the multigraph.
    ///
    /// # Returns
    ///
    /// * A shared pointer to the multigraph.
    fn get_multigraph(&self) -> Rc<RefCell<Multigraph<NM, CM>>>;
}

/// Attempts to make a hop (i.e., a transmission between nodes) for the given route stage and bundle,
/// checking potential contacts to determine the best hop.
///
/// # Parameters
///
/// * `first_contact_index` - The index of the first contact to consider (lazy pruning).
/// * `sndr_route` - A reference-counted, mutable `RouteStage` that represents the sender's current route.
/// * `bundle` - A reference to the `Bundle` that is being routed.
/// * `contacts` - A vector of reference-counted, mutable `Contact`s representing available transmission opportunities.
/// * `tx_node` - A reference-counted, mutable `Node` representing the transmitting node.
/// * `rx_node` - A reference-counted, mutable `Node` representing the receiving node.
///
/// # Returns
///
/// An `Option` containing a `RouteStage` if a suitable hop is found, or `None` if no valid hop is available.
fn try_make_hop<NM: NodeManager, CM: ContactManager>(
    first_contact_index: usize,
    sndr_route: &SharedRouteStage<NM, CM>,
    _bundle: &Bundle,
    contacts: &[Rc<RefCell<Contact<NM, CM>>>],
    tx_node: &Rc<RefCell<Node<NM>>>,
    rx_node: &Rc<RefCell<Node<NM>>>,
) -> Option<RouteStage<NM, CM>> {
    let mut index = 0;
    let mut final_data = ContactManagerTxData {
        tx_start: 0.0,
        tx_end: 0.0,
        expiration: 0.0,
        rx_start: Date::MAX,
        rx_end: Date::MAX,
    };

    // If bundle processing is enabled, a mutable bundle copy is required to be attached to the RouteStage.
    #[cfg(feature = "node_proc")]
    let mut bundle_to_consider = sndr_route.borrow().bundle.clone();
    #[cfg(not(feature = "node_proc"))]
    let bundle_to_consider = _bundle;

    let sndr_route_borrowed = sndr_route.borrow();

    for (idx, contact) in contacts.iter().enumerate().skip(first_contact_index) {
        let contact_borrowed = contact.borrow();

        #[cfg(feature = "contact_suppression")]
        if contact_borrowed.suppressed {
            continue;
        }

        if contact_borrowed.info.start > final_data.rx_end {
            break;
        }

        #[cfg(feature = "node_proc")]
        let sending_time = tx_node
            .borrow()
            .manager
            .dry_run_process(sndr_route_borrowed.at_time, &mut bundle_to_consider);
        #[cfg(not(feature = "node_proc"))]
        let sending_time = sndr_route_borrowed.at_time;

        if let Some(hop) = contact_borrowed.manager.dry_run_tx(
            &contact_borrowed.info,
            sending_time,
            &bundle_to_consider,
        ) {
            #[cfg(feature = "node_tx")]
            if !tx_node.borrow().manager.dry_run_tx(
                sending_time,
                hop.tx_start,
                hop.tx_end,
                &bundle_to_consider,
            ) {
                continue;
            }

            if hop.rx_end < final_data.rx_end {
                #[cfg(feature = "node_rx")]
                if !rx_node
                    .borrow()
                    .manager
                    .dry_run_rx(hop.rx_start, hop.rx_end, _bundle)
                {
                    continue;
                }

                final_data = hop;
                index = idx;
            }
        }
    }

    if final_data.rx_end < Date::MAX {
        let seleted_contact = &contacts[index];
        let mut route_proposition: RouteStage<NM, CM> = RouteStage::new(
            final_data.rx_end,
            seleted_contact.borrow().get_rx_node(),
            Some(ViaHop {
                contact: seleted_contact.clone(),
                parent_route: sndr_route.clone(),
                tx_node: tx_node.clone(),
                rx_node: rx_node.clone(),
            }),
            #[cfg(feature = "node_proc")]
            bundle_to_consider,
        );

        route_proposition.hop_count = sndr_route_borrowed.hop_count + 1;
        route_proposition.cumulative_delay =
            sndr_route_borrowed.cumulative_delay + final_data.rx_end - final_data.tx_end;
        route_proposition.expiration = Date::min(
            final_data.expiration - sndr_route_borrowed.cumulative_delay,
            sndr_route_borrowed.expiration,
        );

        return Some(route_proposition);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::Bundle;
    use crate::contact::Contact;
    use crate::contact_manager::legacy::evl::EVLManager;
    use crate::node::Node;
    use crate::node_manager::NodeManager;
    use crate::node_manager::none::NoManagement;
    use crate::pathfinding::test_helpers::*;
    use crate::route_stage::RouteStage;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[track_caller]
    fn run_hop<NM: NodeManager>(
        first_contact_index: usize,
        source: &SharedRouteStage<NM, EVLManager>,
        bundle: &Bundle,
        contacts: &[Rc<RefCell<Contact<NM, EVLManager>>>],
        tx: &Rc<RefCell<Node<NM>>>,
        rx: &Rc<RefCell<Node<NM>>>,
    ) -> Option<RouteStage<NM, EVLManager>> {
        try_make_hop(first_contact_index, source, bundle, contacts, tx, rx)
    }

    #[test]
    fn test_empty_contacts() {
        // let bundle = make_bundle(1, 1, 50.0, 2000.0);
        // let source = make_source::<NoManagement>(0.0, 0, &bundle);
        // let tx = make_node_rc(0, "A", NoManagement {});
        // let rx = make_node_rc(1, "B", NoManagement {});
        //                                     |
        //                                     v
        let ctx = make_hop_context(50.0);

        let result: Option<RouteStage<NoManagement, EVLManager>> =
            run_hop(0, &ctx.source, &ctx.bundle, &[], &ctx.tx, &ctx.rx);

        assert!(
            result.is_none(),
            "TEST FAILED: Expected None when contacts list is empty."
        );
    }

    #[test]
    fn test_first_contact_index_beyond_slice() {
        let ctx = make_hop_context(50.0);
        let contacts = vec![make_contact_rc::<NoManagement>(
            0, 1, 0.0, 200.0, 100.0, 1.0,
        )];

        let result: Option<RouteStage<NoManagement, EVLManager>> =
            run_hop(1, &ctx.source, &ctx.bundle, &contacts, &ctx.tx, &ctx.rx);

        assert!(
            result.is_none(),
            "TEST FAILED: Expected None when first_contact_index is beyond the slice."
        );
    }

    #[test]
    fn test_bundle_too_large() {
        let ctx = make_hop_context(999_999.0);
        let contacts = vec![make_contact_rc::<NoManagement>(
            0, 1, 0.0, 200.0, 100.0, 1.0,
        )];

        let result = run_hop(0, &ctx.source, &ctx.bundle, &contacts, &ctx.tx, &ctx.rx);

        assert!(
            result.is_none(),
            "TEST FAILED: Expected None when the bundle size exceeds contact capacity."
        );
    }

    #[test]
    fn test_single_contact_valid() {
        let ctx = make_hop_context(50.0);
        let contacts = vec![make_contact_rc::<NoManagement>(
            0, 1, 0.0, 200.0, 100.0, 1.0,
        )];

        let result = run_hop(0, &ctx.source, &ctx.bundle, &contacts, &ctx.tx, &ctx.rx);

        assert!(
            result.is_some(),
            "TEST FAILED: Expected Some when the contact is valid and the bundle size is within contact capacity."
        );
    }

    #[test]
    fn test_first_contact_index_skips_valid_contact() {
        let ctx = make_hop_context(10.0);
        let contact_skipped = make_contact_rc::<NoManagement>(0, 1, 0.0, 200.0, 100.0, 1.0);
        let contact_used = make_contact_rc::<NoManagement>(0, 1, 0.0, 200.0, 100.0, 5.0);

        let result = run_hop(
            1,
            &ctx.source,
            &ctx.bundle,
            &[contact_skipped, contact_used],
            &ctx.tx,
            &ctx.rx,
        );

        assert!(
            result.is_some(),
            "TEST FAILED: Expected Some from contact at index 1."
        );
        let route = result.unwrap();
        assert_eq!(
            route.at_time, 5.1,
            "TEST FAILED: Expected arrival 5.1 from contact at index 1, not 1.1 from skipped contact (got {}).",
            route.at_time
        );
    }

    #[cfg(feature = "contact_suppression")]
    #[test]
    fn test_all_contacts_suppressed() {
        let ctx = make_hop_context(50.0);
        let contact1 = make_contact_rc::<NoManagement>(0, 1, 0.0, 200.0, 100.0, 1.0);
        let contact2 = make_contact_rc::<NoManagement>(0, 1, 20.0, 100.0, 50.0, 1.0);
        let contact3 = make_contact_rc::<NoManagement>(0, 1, 10.0, 300.0, 100.0, 1.0);
        contact1.borrow_mut().suppressed = true;
        contact2.borrow_mut().suppressed = true;
        contact3.borrow_mut().suppressed = true;

        let result = run_hop(
            0,
            &ctx.source,
            &ctx.bundle,
            &[contact1, contact2, contact3],
            &ctx.tx,
            &ctx.rx,
        );

        assert!(
            result.is_none(),
            "TEST FAILED: Expected None when all contacts are suppressed."
        );
    }

    #[cfg(feature = "contact_suppression")]
    #[test]
    fn test_partial_suppression_uses_valid_contact() {
        let ctx = make_hop_context(10.0);
        let contact_suppressed = make_contact_rc::<NoManagement>(0, 1, 0.0, 200.0, 100.0, 1.0);
        let contact_valid = make_contact_rc::<NoManagement>(0, 1, 0.0, 200.0, 100.0, 2.0);
        contact_suppressed.borrow_mut().suppressed = true;

        let result = run_hop(
            0,
            &ctx.source,
            &ctx.bundle,
            &[contact_suppressed, contact_valid],
            &ctx.tx,
            &ctx.rx,
        );

        assert!(
            result.is_some(),
            "TEST FAILED: Expected Some from non-suppressed contact."
        );
        let route = result.unwrap();
        assert_eq!(
            route.at_time, 2.1,
            "TEST FAILED: Expected arrival 2.1 from non-suppressed contact (got {}).",
            route.at_time
        );
    }

    #[cfg(feature = "node_tx")]
    #[test]
    fn test_node_tx_refusing() {
        let bundle = make_bundle(1, 1, 1.0, 2000.0);
        let source = make_source::<MockNodeManager>(0.0, 0, &bundle);
        let tx = make_node_rc(0, "A", MockNodeManager::refusing_tx());
        let rx = make_node_rc(1, "B", MockNodeManager::accepting());
        let contacts = vec![make_contact_rc::<MockNodeManager>(
            0, 1, 0.0, 2000.0, 100.0, 1.0,
        )];

        let result = run_hop(0, &source, &bundle, &contacts, &tx, &rx);

        assert!(
            result.is_none(),
            "TEST FAILED: Expected None when tx node refuses to emit."
        );
    }

    #[cfg(feature = "node_rx")]
    #[test]
    fn test_node_rx_refusing() {
        let bundle = make_bundle(1, 1, 1.0, 2000.0);
        let source = make_source::<MockNodeManager>(0.0, 0, &bundle);
        let tx = make_node_rc(0, "A", MockNodeManager::accepting());
        let rx = make_node_rc(1, "B", MockNodeManager::refusing_rx());
        let contacts = vec![make_contact_rc::<MockNodeManager>(
            0, 1, 0.0, 2000.0, 100.0, 1.0,
        )];

        let result = run_hop(0, &source, &bundle, &contacts, &tx, &rx);

        assert!(
            result.is_none(),
            "TEST FAILED: Expected None when rx node refuses to receive."
        );
    }

    #[cfg(feature = "node_proc")]
    #[test]
    fn test_node_proc_delay() {
        let bundle = make_bundle(1, 1, 10.0, 2000.0);
        let source = make_source::<MockNodeManager>(0.0, 0, &bundle);
        let tx = make_node_rc(0, "A", MockNodeManager::processing(2.0));
        let rx = make_node_rc(1, "B", MockNodeManager::accepting());
        let contacts = vec![make_contact_rc::<MockNodeManager>(
            0, 1, 0.0, 2000.0, 100.0, 1.0,
        )];

        let result = run_hop(0, &source, &bundle, &contacts, &tx, &rx);

        assert!(
            result.is_some(),
            "TEST FAILED: Expected Some even with node processing delay."
        );
        let route = result.unwrap();
        // without node_proc : sending_time = 0.0 -> tx_end = 0.1 -> arrival = 1.1
        // with node_proc : sending_time = 2.0 -> tx_end = 2.1 -> arrival = 3.1
        assert_eq!(
            route.at_time, 3.1,
            "TEST FAILED: Arrival should account for the 2s node processing delay (expected 3.1, got {}).",
            route.at_time
        );
    }

    #[test]
    fn test_best_contact_selected_1_hop() {
        let bundle = make_bundle(1, 1, 100.0, 2000.0);
        let source = make_source::<NoManagement>(5.0, 0, &bundle);
        let tx = make_node_rc(0, "A", NoManagement {});
        let rx = make_node_rc(1, "B", NoManagement {});
        // Contact A : arrival = 11.0
        let contact_a = make_contact_rc::<NoManagement>(0, 1, 0.0, 50.0, 100.0, 5.0);
        // Contact B : arrival = 8.0 -> should be the one returned
        let contact_b = make_contact_rc::<NoManagement>(0, 1, 0.0, 200.0, 100.0, 2.0);
        // Contact C : start = 10.0 > arrival(8.0) -> pruned
        let contact_c = make_contact_rc::<NoManagement>(0, 1, 10.0, 100.0, 50.0, 1.0);
        // Contact D : start = 20.0 > arrival(8.0) -> pruned
        let contact_d = make_contact_rc::<NoManagement>(0, 1, 20.0, 30.0, 100.0, 0.5);

        let result = run_hop(
            0,
            &source,
            &bundle,
            &[contact_a, contact_b, contact_c, contact_d],
            &tx,
            &rx,
        );

        assert!(
            result.is_some(),
            "TEST FAILED: Expected Some, at least one contact should be valid."
        );
        let route = result.unwrap();

        // Contact B should have been selected : arrival = tx_end(6.0) + delay(2.0) = 8.0
        assert_eq!(
            route.at_time, 8.0,
            "TEST FAILED: Expected arrival 8.0 from contact B (got {}).",
            route.at_time
        );
        assert_eq!(
            route.hop_count, 1,
            "TEST FAILED: Expected hop_count = 1 (got {}).",
            route.hop_count
        );
        assert_eq!(
            route.cumulative_delay, 2.0,
            "TEST FAILED: Expected cumulative_delay=2.0 from contact B delay (got {}).",
            route.cumulative_delay
        );
        assert_eq!(
            route.expiration, 200.0,
            "TEST FAILED: Expected expiration = 200.0 from contact B end (got {}).",
            route.expiration
        );
        assert!(
            route.via.is_some(),
            "TEST FAILED: Expected a ViaHop to be set."
        );
    }

    #[test]
    fn test_best_contact_selected_2_hops() {
        let ctx = make_hop_context(100.0);
        // We set the expiration on the source to test that min(contact.end - cumulative_delay, source.expiration) works
        ctx.source.borrow_mut().expiration = 150.0;

        // Contact A : arrival = 2.0 -> the best one
        let contact_a = make_contact_rc::<NoManagement>(0, 1, 0.0, 200.0, 100.0, 1.0);
        // Contact B : arrival = 6.0
        let contact_b = make_contact_rc::<NoManagement>(0, 1, 0.0, 200.0, 100.0, 5.0);

        let hop1 = run_hop(
            0,
            &ctx.source,
            &ctx.bundle,
            &[contact_a, contact_b],
            &ctx.tx,
            &ctx.rx,
        )
        .expect("TEST FAILED: Hop 1 should succeed.");

        assert_eq!(
            hop1.at_time, 2.0,
            "Hop 1 FAILED: Expected arrival 2.0 (got {}).",
            hop1.at_time
        );
        assert_eq!(
            hop1.hop_count, 1,
            "Hop 1 FAILED: Expected hop_count = 1 (got {}).",
            hop1.hop_count
        );
        assert_eq!(
            hop1.cumulative_delay, 1.0,
            "Hop 1 FAILED: Expected cumulative_delay = 1.0 (got {}).",
            hop1.cumulative_delay
        );
        // min(contact_a.end(200.0) - cumulative_delay(0.0), source.expiration(150.0)) = 150.0
        assert_eq!(
            hop1.expiration, 150.0,
            "Hop 1 FAILED: Expected expiration = 150.0 limited by source.expiration (got {}).",
            hop1.expiration
        );

        // We take the result of the first hop as a new source
        let source2: SharedRouteStage<NoManagement, EVLManager> = Rc::new(RefCell::new(hop1));
        let tx1 = make_node_rc(1, "B", NoManagement {});
        let rx2 = make_node_rc(2, "C", NoManagement {});

        // Contacts with end = 1000.0 so that source2.expiration is the limiting factor
        // Contact C : arrival = 3.5 -> the best one
        let contact_c = make_contact_rc::<NoManagement>(1, 2, 0.0, 1000.0, 100.0, 0.5);
        // Contact D : arrival = 5.0
        let contact_d = make_contact_rc::<NoManagement>(1, 2, 0.0, 1000.0, 100.0, 2.0);

        let hop2 = run_hop(
            0,
            &source2,
            &ctx.bundle,
            &[contact_c, contact_d],
            &tx1,
            &rx2,
        )
        .expect("TEST FAILED: Hop 2 should succeed.");

        assert_eq!(
            hop2.at_time, 3.5,
            "Hop 2 FAILED: Expected arrival 3.5 (got {}).",
            hop2.at_time
        );
        assert_eq!(
            hop2.hop_count, 2,
            "Hop 2 FAILED: Expected hop_count=2 (got {}).",
            hop2.hop_count
        );
        assert_eq!(
            hop2.cumulative_delay, 1.5,
            "Hop 2 FAILED: Expected cumulative_delay=1.5 (got {}).",
            hop2.cumulative_delay
        );
        // min(contact_c.end(1000.0) - cumulative_delay(1.0), source2.expiration(150.0)) = 150.0
        assert_eq!(
            hop2.expiration, 150.0,
            "Hop 2 FAILED: Expected expiration=150.0 limited by propagated source.expiration (got {}).",
            hop2.expiration
        );
        assert!(
            hop2.via.is_some(),
            "Hop 2 FAILED: Expected a ViaHop to be set."
        );
    }
}
