use crate::bundle::Bundle;
use crate::contact::Contact;
use crate::contact::ContactInfo;
use crate::contact_manager::legacy::evl::EVLManager;
use crate::contact_plan::ContactPlan;
use crate::multigraph::Multigraph;
use crate::node::Node;
use crate::node::NodeInfo;
use crate::node_manager::none::NoManagement;
use crate::pathfinding::ASABRError;
use crate::pathfinding::NodeID;
use crate::pathfinding::PathFindingOutput;
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) fn make_node(id: u16, name: &str) -> Node<NoManagement> {
    Node::try_new(
        NodeInfo {
            id,
            name: name.into(),
            excluded: false,
        },
        NoManagement {},
    )
    .unwrap()
}

pub(crate) fn make_contact(
    tx: u16,
    rx: u16,
    start: f64,
    end: f64,
    rate: f64,
    delay: f64,
) -> Contact<NoManagement, EVLManager> {
    Contact::try_new(
        ContactInfo::new(tx, rx, start, end),
        EVLManager::new(rate, delay),
    )
    .unwrap_or_else(|| panic!("Contact failed"))
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
        vec![make_node(0, "A"), make_node(1, "B"), make_node(2, "C")],
        vec![
            make_contact(0, 1, 0.0, 2000.0, 100.0, 1.0),
            make_contact(1, 2, 0.0, 2000.0, 100.0, 1.0),
        ],
        None,
    )?)?)))
}

pub(crate) fn five_contact_graph_test()
-> Result<Rc<RefCell<Multigraph<NoManagement, EVLManager>>>, ASABRError> {
    Ok(Rc::new(RefCell::new(Multigraph::new(ContactPlan::new(
        vec![
            make_node(0, "A"),
            make_node(1, "B"),
            make_node(2, "C"),
            make_node(3, "D"),
        ],
        vec![
            make_contact(0, 1, 0.0, 2000.0, 100.0, 0.01),
            make_contact(1, 2, 0.0, 2000.0, 100.0, 1.0),
            make_contact(0, 3, 0.0, 2000.0, 100.0, 0.1),
            make_contact(3, 2, 0.0, 2000.0, 100.0, 0.01),
            make_contact(0, 2, 0.0, 2000.0, 100.0, 10.0),
        ],
        None,
    )?)?)))
}

pub(crate) fn exemple_1_graph()
-> Result<Rc<RefCell<Multigraph<NoManagement, EVLManager>>>, ASABRError> {
    Ok(Rc::new(RefCell::new(Multigraph::new(ContactPlan::new(
        vec![
            make_node(0, "source"),
            make_node(1, "from_C0"),
            make_node(2, "from_C2_C1"),
            make_node(3, "from_C3"),
        ],
        vec![
            make_contact(0, 1, 0.0, 10.0, 1.0, 0.0),
            make_contact(0, 2, 25.0, 35.0, 1.0, 0.0),
            make_contact(1, 2, 10.0, 20.0, 1.0, 0.0),
            make_contact(2, 3, 30.0, 40.0, 1.0, 0.0),
        ],
        None,
    )?)?)))
}

pub(crate) fn exemple_2_graph()
-> Result<Rc<RefCell<Multigraph<NoManagement, EVLManager>>>, ASABRError> {
    Ok(Rc::new(RefCell::new(Multigraph::new(ContactPlan::new(
        vec![
            make_node(0, "source"),
            make_node(1, "from_C0"),
            make_node(2, "from_C2_C1"),
            make_node(3, "from_C3"),
            make_node(4, "from_C4"),
        ],
        vec![
            make_contact(0, 1, 0.0, 10.0, 1.0, 0.0),
            make_contact(0, 2, 25.0, 35.0, 1.0, 0.0),
            make_contact(1, 2, 10.0, 20.0, 1.0, 0.0),
            make_contact(2, 3, 20.0, 40.0, 1.0, 0.0),
            make_contact(3, 4, 50.0, 60.0, 1.0, 0.0),
        ],
        None,
    )?)?)))
}
