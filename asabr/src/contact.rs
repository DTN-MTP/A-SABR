use crate::contact_manager::ContactManager;
use crate::parse_transparent;
use crate::types::{Date, NodeID, TimeInterval};

use core::cmp::Ordering;
extern crate alloc;

/// Represents basic information about a contact between two nodes.
#[derive(Clone, Copy, Debug)]
pub struct ContactInfo {
    ///The ID of the transmitting node.
    pub tx_node_id: NodeID,
    /// The ID of the receiving node.
    pub rx_node_id: NodeID,
    /// The start time of the contact.
    pub start: Date,
    /// The end time of the contact.
    pub end: Date,
}

parse_transparent!(ContactInfo, (NodeID, NodeID, Date, Date));

impl From<(NodeID, NodeID, Date, Date)> for ContactInfo {
    fn from((tx_node_id, rx_node_id, start, end): (NodeID, NodeID, Date, Date)) -> Self {
        ContactInfo {
            tx_node_id,
            rx_node_id,
            start,
            end,
        }
    }
}

impl From<ContactInfo> for TimeInterval {
    fn from(value: ContactInfo) -> Self {
        TimeInterval {
            start: value.start,
            end: value.end,
        }
    }
}

impl ContactInfo {
    /// Creates a new `ContactInfo` instance.
    ///
    /// # Parameters
    ///
    /// * `tx_node_id` - The ID of the transmitting node.
    /// * `rx_node_id` - The ID of the receiving node.
    /// * `start` - The start time of the contact.
    /// * `end` - The end time of the contact.
    ///
    /// # Returns
    ///
    /// * `Self` - A new instance of `ContactInfo`.
    pub fn new(tx_node_id: NodeID, rx_node_id: NodeID, start: Date, end: Date) -> Self {
        Self {
            tx_node_id,
            rx_node_id,
            start,
            end,
        }
    }

    /// Checks if the contact is valid based on its start and end times.
    ///
    /// # Returns
    ///
    /// * `bool` - Returns `true` if the start time is before the end time; otherwise, returns `false`.
    fn try_init(&self) -> bool {
        self.start < self.end
    }
}

/// Represents a contact with associated management information.
///
///  # Type Parameters
/// - `NM`: A type implementing the `NodeManager` trait, responsible for managing the
///   node's operations.
/// - `CM`: A type implementing the `ContactManager` trait, responsible for managing the
///   contact's operations.
#[derive(Debug)]
pub struct Contact<CM: ContactManager> {
    /// This contact relevancy window
    pub lifespan: TimeInterval,
    /// The manager handling the contact's operations.
    pub manager: CM,
    #[cfg(feature = "contact_suppression")]
    /// Suppression option for path construction (compilation option).
    pub suppressed: bool,
}

impl<CM: ContactManager> Contact<CM> {
    /// Creates a new `Contact` instance if the contact information and manager are valid.
    ///
    /// # Parameters
    ///
    /// * `info` - The contact information.
    /// * `manager` - The contact manager.
    ///
    /// # Returns
    ///
    /// * `Option<Self>` - Returns `Some(Contact)` if creation was successful; otherwise, returns `None`.
    pub fn try_new(info: ContactInfo, mut manager: CM) -> Option<(Self, usize, usize)> {
        if info.try_init() && manager.try_init(&info) {
            return Some((
                Contact {
                    lifespan: TimeInterval {
                        start: info.start,
                        end: info.end,
                    },
                    manager,
                    #[cfg(feature = "contact_suppression")]
                    suppressed: false,
                },
                info.tx_node_id.into(),
                info.rx_node_id.into(),
            ));
        }
        None
    }

    /// Compare two contacts by start time.
    pub fn cmp_by_start(&self, other: &Self) -> Ordering {
        self.lifespan
            .start
            .partial_cmp(&other.lifespan.start)
            .unwrap_or(Ordering::Equal)
    }
}

impl<CM: ContactManager> AsRef<Self> for Contact<CM> {
    fn as_ref(&self) -> &Self {
        self
    }
}
