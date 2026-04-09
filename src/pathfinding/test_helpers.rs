use crate::bundle::Bundle;
use crate::contact::Contact;
use crate::contact::ContactInfo;
use crate::contact_manager::legacy::evl::EVLManager;
use crate::contact_plan::ContactPlan;
use crate::multigraph::Multigraph;
use crate::node::Node;
use crate::node::NodeInfo;
use crate::node_manager::NodeManager;
use crate::node_manager::none::NoManagement;
use crate::pathfinding::ASABRError;
use crate::pathfinding::NodeID;
use crate::pathfinding::PathFindingOutput;
use crate::route_stage::{RouteStage, SharedRouteStage};
use crate::types::Date;
use std::cell::RefCell;
use std::rc::Rc;

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
            process_output: 0.0,
        }
    }
    #[cfg(feature = "node_tx")]
    pub(crate) fn refusing_tx() -> Self {
        Self {
            tx_ok: false,
            rx_ok: true,
            process_output: 0.0,
        }
    }
    #[cfg(feature = "node_rx")]
    pub(crate) fn refusing_rx() -> Self {
        Self {
            tx_ok: true,
            rx_ok: false,
            process_output: 0.0,
        }
    }
    #[cfg(feature = "node_proc")]
    pub(crate) fn processing(process_output: Date) -> Self {
        Self {
            tx_ok: true,
            rx_ok: true,
            process_output,
        }
    }
}

impl NodeManager for MockNodeManager {
    #[cfg(feature = "node_proc")]
    fn dry_run_process(&self, _at_time: Date, _bundle: &mut Bundle) -> Date {
        self.process_output
    }
    #[cfg(feature = "node_proc")]
    fn schedule_process(&self, _at_time: Date, _bundle: &mut Bundle) -> Date {
        unimplemented!("Not needed in tests")
    }
    #[cfg(feature = "node_tx")]
    fn dry_run_tx(&self, _: Date, _: Date, _: Date, _: &Bundle) -> bool {
        self.tx_ok
    }
    #[cfg(feature = "node_tx")]
    fn schedule_tx(&mut self, _: Date, _: Date, _: Date, _: &Bundle) -> bool {
        unimplemented!("Not needed in tests")
    }
    #[cfg(feature = "node_rx")]
    fn dry_run_rx(&self, _: Date, _: Date, _: &Bundle) -> bool {
        self.rx_ok
    }
    #[cfg(feature = "node_rx")]
    fn schedule_rx(&mut self, _: Date, _: Date, _: &Bundle) -> bool {
        unimplemented!("Not needed in tests")
    }
}

pub(crate) fn make_node<NM: NodeManager>(id: u16, name: &str, nm: NM) -> Node<NM> {
    Node::try_new(
        NodeInfo {
            id,
            name: name.into(),
            excluded: false,
        },
        nm,
    )
    .unwrap()
}

pub(crate) fn make_node_rc<NM: NodeManager>(id: u16, name: &str, nm: NM) -> Rc<RefCell<Node<NM>>> {
    Rc::new(RefCell::new(make_node(id, name, nm)))
}

pub(crate) fn make_contact<NM: NodeManager>(
    tx: u16,
    rx: u16,
    start: f64,
    end: f64,
    rate: f64,
    delay: f64,
) -> Contact<NM, EVLManager> {
    Contact::try_new(
        ContactInfo::new(tx, rx, start, end),
        EVLManager::new(rate, delay),
    )
    .expect("Contact creation failed")
}

pub(crate) fn make_contact_rc<NM: NodeManager>(
    tx: u16,
    rx: u16,
    start: f64,
    end: f64,
    rate: f64,
    delay: f64,
) -> Rc<RefCell<Contact<NM, EVLManager>>> {
    Rc::new(RefCell::new(make_contact(tx, rx, start, end, rate, delay)))
}

pub(crate) fn make_source<NM: NodeManager>(
    at_time: Date,
    node_id: u16,
    _bundle: &Bundle,
) -> SharedRouteStage<NM, EVLManager> {
    Rc::new(RefCell::new(RouteStage::new(
        at_time,
        node_id,
        None,
        #[cfg(feature = "node_proc")]
        _bundle.clone(),
    )))
}

pub(crate) fn make_bundle(dest: NodeID, priority: i8, size: f64, expiration: f64) -> Bundle {
    Bundle {
        source: 0,
        destinations: vec![dest],
        priority,
        size,
        expiration,
    }
}

pub(crate) fn assert_time_hop(
    res: &PathFindingOutput<NoManagement, EVLManager>,
    dest: usize,
    expected_time: f64,
    expected_hop: u16,
    distance: &str,
) {
    let r = res.by_destination[dest]
        .as_ref()
        .unwrap_or_else(|| panic!("{distance} : No route found to node {dest}"))
        .borrow();
    assert_eq!(
        r.at_time, expected_time,
        "{distance} : Arrival time should be {expected_time}"
    );
    assert_eq!(
        r.hop_count, expected_hop,
        "{distance} : Should be {expected_hop} hops"
    );
}

pub(crate) fn unit_graph_test()
-> Result<Rc<RefCell<Multigraph<NoManagement, EVLManager>>>, ASABRError> {
    Ok(Rc::new(RefCell::new(Multigraph::new(ContactPlan::new(
        vec![
            make_node(0, "A", NoManagement {}),
            make_node(1, "B", NoManagement {}),
            make_node(2, "C", NoManagement {}),
        ],
        vec![
            make_contact::<NoManagement>(0, 1, 0.0, 2000.0, 100.0, 1.0),
            make_contact::<NoManagement>(1, 2, 0.0, 2000.0, 100.0, 1.0),
        ],
        None,
    )?)?)))
}

pub(crate) fn five_contact_graph_test()
-> Result<Rc<RefCell<Multigraph<NoManagement, EVLManager>>>, ASABRError> {
    Ok(Rc::new(RefCell::new(Multigraph::new(ContactPlan::new(
        vec![
            make_node(0, "A", NoManagement {}),
            make_node(1, "B", NoManagement {}),
            make_node(2, "C", NoManagement {}),
            make_node(3, "D", NoManagement {}),
        ],
        vec![
            make_contact::<NoManagement>(0, 1, 0.0, 2000.0, 100.0, 0.01),
            make_contact::<NoManagement>(1, 2, 0.0, 2000.0, 100.0, 1.0),
            make_contact::<NoManagement>(0, 3, 0.0, 2000.0, 100.0, 0.1),
            make_contact::<NoManagement>(3, 2, 0.0, 2000.0, 100.0, 0.01),
            make_contact::<NoManagement>(0, 2, 0.0, 2000.0, 100.0, 10.0),
        ],
        None,
    )?)?)))
}

pub(crate) fn exemple_1_graph()
-> Result<Rc<RefCell<Multigraph<NoManagement, EVLManager>>>, ASABRError> {
    Ok(Rc::new(RefCell::new(Multigraph::new(ContactPlan::new(
        vec![
            make_node(0, "source", NoManagement {}),
            make_node(1, "from_C0", NoManagement {}),
            make_node(2, "from_C2_C1", NoManagement {}),
            make_node(3, "from_C3", NoManagement {}),
        ],
        vec![
            make_contact::<NoManagement>(0, 1, 0.0, 10.0, 1.0, 0.0),
            make_contact::<NoManagement>(0, 2, 25.0, 35.0, 1.0, 0.0),
            make_contact::<NoManagement>(1, 2, 10.0, 20.0, 1.0, 0.0),
            make_contact::<NoManagement>(2, 3, 30.0, 40.0, 1.0, 0.0),
        ],
        None,
    )?)?)))
}

pub(crate) fn exemple_2_graph()
-> Result<Rc<RefCell<Multigraph<NoManagement, EVLManager>>>, ASABRError> {
    Ok(Rc::new(RefCell::new(Multigraph::new(ContactPlan::new(
        vec![
            make_node(0, "source", NoManagement {}),
            make_node(1, "from_C0", NoManagement {}),
            make_node(2, "from_C2_C1", NoManagement {}),
            make_node(3, "from_C3", NoManagement {}),
            make_node(4, "from_C4", NoManagement {}),
        ],
        vec![
            make_contact::<NoManagement>(0, 1, 0.0, 10.0, 1.0, 0.0),
            make_contact::<NoManagement>(0, 2, 25.0, 35.0, 1.0, 0.0),
            make_contact::<NoManagement>(1, 2, 10.0, 20.0, 1.0, 0.0),
            make_contact::<NoManagement>(2, 3, 20.0, 40.0, 1.0, 0.0),
            make_contact::<NoManagement>(3, 4, 50.0, 60.0, 1.0, 0.0),
        ],
        None,
    )?)?)))
}

pub(crate) struct HopContext<NM: NodeManager> {
    pub bundle: Bundle,
    pub source: SharedRouteStage<NM, EVLManager>,
    pub nodes: Vec<Rc<RefCell<Node<NM>>>>,
}

pub(crate) fn make_hop_context(size: f64) -> HopContext<NoManagement> {
    let bundle = make_bundle(1, 1, size, 2000.0);
    let source = make_source::<NoManagement>(0.0, 0, &bundle);
    let tx = make_node_rc(0, "A", NoManagement {});
    let rx = make_node_rc(1, "B", NoManagement {});
    let nodes = vec![tx, rx];
    HopContext {
        bundle,
        source,
        nodes,
    }
}
