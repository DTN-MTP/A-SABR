extern crate alloc;

use crate::{
    bundle::Bundle,
    contact::{Contact, ContactInfo},
    contact_manager::legacy::evl::EVLManager,
    contact_plan::{ContactPlan, RealNode, asabr_file_lexer::parse_from_iter},
    mk_graph_pathfinding,
    multigraph::Multigraph,
    node::{Node, NodeInfo},
    node_manager::{NodeManager, none::NoManagement},
    pathfinding::{ASABRError, PathFindingOutput, Pathfinding},
    paths::PathFragment,
    types::{Date, NodeID},
};
use alloc::vec;

#[derive(Debug)]
pub(crate) struct MockNodeManager {
    pub tx_ok: bool,
    pub rx_ok: bool,
    pub process_output: Date,
}

impl MockNodeManager {
    pub(crate) fn accepting() -> Self {
        Self {
            tx_ok: true,
            rx_ok: true,
            process_output: 0,
        }
    }
    pub(crate) fn refusing_tx() -> Self {
        Self {
            tx_ok: false,
            rx_ok: true,
            process_output: 0,
        }
    }
    pub(crate) fn refusing_rx() -> Self {
        Self {
            tx_ok: true,
            rx_ok: false,
            process_output: 0,
        }
    }
    pub(crate) fn processing(process_output: Date) -> Self {
        Self {
            tx_ok: true,
            rx_ok: true,
            process_output,
        }
    }
}

impl NodeManager for MockNodeManager {
    fn accept(&self, _bundle: &Bundle, _time: crate::types::TimeInterval, _sender: NodeID) -> bool {
        self.rx_ok
    }

    fn dry_run_retention(
        &self,
        _bundle: &Bundle,
        _reception: crate::types::TimeInterval,
        _sender: NodeID,
        _transmition: crate::types::TimeInterval,
        _next: NodeID,
    ) -> bool {
        self.tx_ok
    }

    fn dry_run_multi(
        &self,
        _bundle: &Bundle,
        _reception: crate::types::TimeInterval,
        _sender: NodeID,
        transmitions: &[(crate::types::TimeInterval, NodeID)],
    ) -> Option<usize> {
        if self.tx_ok {
            Some(transmitions.len())
        } else {
            if self.rx_ok { Some(0) } else { None }
        }
    }

    fn commit(
        &mut self,
        _bundle: &Bundle,
        _reception: crate::types::TimeInterval,
        _sender: NodeID,
        transmitions: &[(crate::types::TimeInterval, NodeID)],
    ) -> Result<(), ASABRError> {
        if !self.rx_ok {
            panic!("Cannot receive a paquet!")
        } else if !self.tx_ok && transmitions.len() != 0 {
            panic!("Cannot send a paquet")
        }
        Ok(())
    }

    fn delay(
        &self,
        _bundle: &Bundle,
        _reception: crate::types::TimeInterval,
        _sender: NodeID,
        _next: NodeID,
    ) -> Date {
        self.process_output
    }
}

pub(crate) fn make_vertex<NM: NodeManager>(id: u16, name: &str, nm: NM) -> RealNode<NM> {
    RealNode::Inode(
        Node::try_new(
            NodeInfo {
                id,
                name: name.into(),
                excluded: false,
            },
            nm,
        )
        .unwrap(),
    )
}

pub(crate) fn make_contact(
    tx: u16,
    rx: u16,
    start: i64,
    end: i64,
    rate: f64,
    delay: i64,
) -> (Contact<EVLManager>, usize, usize) {
    Contact::try_new(
        ContactInfo::new(tx, rx, start, end),
        EVLManager::new(rate, delay),
    )
    .expect("Contact creation failed")
}

pub(crate) fn make_source<'id>(
    at_time: Date,
    _node_id: u16,
    _bundle: &Bundle,
) -> PathFragment<'id> {
    PathFragment::new(
        crate::types::TimeInterval {
            start: at_time,
            end: at_time,
        },
        None,
        0,
    )
}

pub(crate) fn make_bundle(dest: NodeID, priority: i8, size: f64, expiration: Date) -> Bundle {
    Bundle {
        source: 0,
        destinations: vec![dest],
        priority,
        size,
        expiration,
    }
}

pub(crate) fn assert_time_hop<'id, 'a>(
    res: &PathFindingOutput<'id, 'a>,
    dest: usize,
    expected_time: i64,
    expected_hop: u16,
    distance: &str,
) {
    assert!(
        res[dest].is_some_and(|path| path.arrival_time.end == expected_time),
        "{distance} : Arrival time should be {expected_time}"
    );
    assert!(
        res[dest].is_some_and(|path| path.hop_count == expected_hop),
        "{distance} : Should be {expected_hop} hops"
    );
}

pub const TEST_GRAPHS: [(&str, &str); 4] = [
    (
        "unit_graph",
        "node 0 A node 1 B node 2 C
     contact 0 1 0 2000 100 1
     contact 1 2 0 2000 100 1",
    ),
    (
        "five_contact",
        "node 0 A node 1 B node 2 C node 3 D
    contact 0 1 0 2000 100 0.01
    contact 1 2 0 2000 100 1
    contact 0 3 0 2000 100 0.1
    contact 3 2 0 2000 100 0.01
    contact 0 2 0 2000 100 10",
    ),
    (
        "exemple 1",
        "node 0 source node 1 from_C0 node 2 from_C2_C1 from_C3
     contact 0 1 0 10 1 0
     contact 0 2 25 35 1 0
     contact 1 2 10 20 1 0
     contact 2 3 30 40 1 0",
    ),
    (
        "exemple 2",
        "node 0 source node 1 from_C0 node 2 from_C2_C1 node 3 from_C3 node 4 from_C4
    contact 0 1 0 10 1 0
    contact 0 2 25 35 1 0
    contact 1 2 10 23 1 0
    contact 2 3 20 40 1 0
    contact 3 4 50 60 1 0",
    ),
];

pub fn for_test_graph<'id, A, P: Pathfinding<'id, NoManagement, EVLManager>>(
    graph_index: usize,
    f: impl FnOnce(&mut Multigraph<'id, NoManagement, EVLManager>, &mut P) -> Result<A, ASABRError>,
) -> Result<A, ASABRError> {
    let graph = TEST_GRAPHS[graph_index].1;
    mk_graph_pathfinding!(graph, finder, NoManagement, EVLManager, P, graph, raw);
    f(&mut graph, &mut finder)
}

// pub(crate) struct HopContext<'id, T: Pathfinding<'id, NM, EVLManager>, NM: NodeManager> {
//     pub bundle: Bundle,
//     pub source: PathFragment<'id, T, NM, EVLManager>,
//     pub nodes: Vec<RealNode<NM>>,
// }

// pub(crate) fn make_hop_context<'id, T: Pathfinding<'id, NoManagement, EVLManager>>(
//     size: f64,
// ) -> HopContext<'id, T, NoManagement> {
//     let bundle = make_bundle(1, 1, size, 2000.0);
//     let source = make_source(0.0, 0, &bundle);
//     let tx = make_vertex(0, "A", NoManagement {});
//     let rx = make_vertex(1, "B", NoManagement {});
//     let nodes = vec![tx, rx];
//     HopContext {
//         bundle,
//         source,
//         nodes,
//     }
// }

/// Creates a graph with vnodes for testing anycast routing.
///
/// Topology (real nodes):
///   A(0) --c0--> B(1) --c1--> C(2)
///   A(0) --c2--> D(3) --c3--> E(4)
///
/// VNode V(5) labels both C(2) and E(4).
///
/// Contact c0: A->B, delay=1.0
/// Contact c1: B->C, delay=1.0
/// Contact c2: A->D, delay=0.5
/// Contact c3: D->E, delay=0.5
///
/// Routing to V(5) should find the path A->D->E (arrival=1.01) over A->B->C (arrival=2.02)
/// because E is reached faster and is part of the same vnode group.
pub(crate) fn vnode_anycast_graph() -> Result<ContactPlan<NoManagement, EVLManager>, ASABRError> {
    let cp = "node 0 A node 1 B node 2 C node 3 D node 4 E
            contact 0 1 0 2000 100 1
            contact 1 2 0 2000 100 1
            contact 0 3 0 2000 100 0.5
            contact 3 4 0 2000 100 0.5"
        .lines();
    parse_from_iter(cp)
}
