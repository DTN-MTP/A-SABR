extern crate alloc;

use crate::{
    bundle::Bundle,
    contact::{Contact, ContactInfo},
    contact_manager::legacy::evl::EVLManager,
    contact_plan::RealNode,
    node::{Node, NodeInfo},
    node_manager::NodeManager,
    pathfinding::ASABRError,
    types::{DataRate, Date, Duration, NodeID, Priority, Volume},
};

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
        } else if !self.tx_ok && !transmitions.is_empty() {
            panic!("Cannot send a paquet")
        }
        Ok(())
    }

    fn process_delay(
        &self,
        _bundle: &Bundle,
        _reception: crate::types::TimeInterval,
        _sender: NodeID,
        _next: NodeID,
    ) -> Date {
        self.process_output
    }
}

pub(crate) fn make_vertex<NM: NodeManager>(id: usize, name: &str, nm: NM) -> RealNode<NM> {
    RealNode::Inode(
        Node::try_new(
            NodeInfo {
                id: id.into(),
                name: name.into(),
                excluded: false,
            },
            nm,
        )
        .unwrap(),
    )
}

pub(crate) fn make_contact(
    tx: usize,
    rx: usize,
    start: Date,
    end: Date,
    rate: DataRate,
    delay: Duration,
) -> (Contact<EVLManager>, usize, usize) {
    Contact::try_new(
        ContactInfo::new(tx.into(), rx.into(), start, end),
        EVLManager::new(rate, delay),
    )
    .expect("Contact creation failed")
}

pub(crate) fn make_bundle(priority: Priority, size: Volume, expiration: Date) -> Bundle {
    Bundle {
        priority,
        size,
        expiration,
    }
}
