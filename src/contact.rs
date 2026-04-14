use crate::contact_manager::ContactManager;
use crate::errors::ASABRError;
use crate::node_manager::NodeManager;
use crate::parsing::{Lexer, Parser};
#[cfg(feature = "contact_work_area")]
use crate::route_stage::SharedRouteStage;
use crate::types::{Date, NodeID, Token};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::marker::PhantomData;
use std::rc::Rc;

/// Represents basic information about a contact between two nodes.
#[derive(Clone, Copy)]
#[cfg_attr(feature = "debug", derive(Debug))]
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
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Contact<NM: NodeManager, CM: ContactManager> {
    /// The basic information about the contact.
    pub info: ContactInfo,
    /// The manager handling the contact's operations.
    pub manager: CM,
    #[cfg(feature = "contact_work_area")]
    /// The work area for managing path construction stages (compilation option).
    pub work_area: Option<SharedRouteStage<NM, CM>>,
    #[cfg(feature = "contact_suppression")]
    /// Suppression option for path construction (compilation option).
    pub suppressed: bool,

    // for compilation
    #[doc(hidden)]
    _phantom_nm: PhantomData<NM>,
}

pub type SharedContact<NM, CM> = Rc<RefCell<Contact<NM, CM>>>;

impl<NM: NodeManager, CM: ContactManager> Contact<NM, CM> {
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
    pub fn try_new(info: ContactInfo, mut manager: CM) -> Option<Self> {
        if info.try_init() && manager.try_init(&info) {
            return Some(Contact {
                info,
                manager,
                #[cfg(feature = "contact_work_area")]
                work_area: None,
                #[cfg(feature = "contact_suppression")]
                suppressed: false,
                // for compilation
                _phantom_nm: PhantomData,
            });
        }
        None
    }

    /// Retrieves the transmitting node's ID.
    ///
    /// # Returns
    ///
    /// * `NodeID` - The ID of the transmitting node.
    #[inline(always)]
    pub fn get_tx_node_id(&self) -> NodeID {
        self.info.tx_node_id
    }

    /// Retrieves the receiving node's ID.
    ///
    /// # Returns
    ///
    /// * `NodeID` - The ID of the receiving node.
    #[inline(always)]
    pub fn get_rx_node_id(&self) -> NodeID {
        self.info.rx_node_id
    }

    /// Compare two contacts by start time.
    pub fn cmp_by_start(&self, other: &Self) -> Ordering {
        self.info
            .start
            .partial_cmp(&other.info.start)
            .unwrap_or(Ordering::Equal)
    }
}

impl<NM: NodeManager, CM: ContactManager> Ord for Contact<NM, CM> {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.info.tx_node_id > other.info.tx_node_id {
            return Ordering::Greater;
        }
        if self.info.tx_node_id < other.info.tx_node_id {
            return Ordering::Less;
        }
        if self.info.rx_node_id > other.info.rx_node_id {
            return Ordering::Greater;
        }
        if self.info.rx_node_id < other.info.rx_node_id {
            return Ordering::Less;
        }
        if self.info.start > other.info.start {
            return Ordering::Greater;
        }
        if self.info.start < other.info.start {
            return Ordering::Less;
        }
        Ordering::Equal
    }
}

impl<NM: NodeManager, CM: ContactManager> PartialOrd for Contact<NM, CM> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<NM: NodeManager, CM: ContactManager> PartialEq for Contact<NM, CM> {
    fn eq(&self, other: &Self) -> bool {
        self.info.tx_node_id == other.info.tx_node_id
            && self.info.rx_node_id == other.info.rx_node_id
            && self.info.start == other.info.start
    }
}
impl<NM: NodeManager, CM: ContactManager> Eq for Contact<NM, CM> {}

impl Parser<ContactInfo> for ContactInfo {
    /// Parses a `ContactInfo` from a lexer.
    ///
    /// # Parameters
    ///
    /// * `lexer` - A mutable reference to a lexer that provides tokens for parsing.
    ///
    /// # Returns
    ///
    /// * `Result<LexerOutput<ContactInfo>, ASABRError>` - The successful parsing state or an error.
    fn parse(lexer: &mut dyn Lexer) -> Result<ContactInfo, ASABRError> {
        let tx_node_id: NodeID = NodeID::parse(lexer)?;

        let rx_node_id: NodeID = NodeID::parse(lexer)?;

        let start: Date = Date::parse(lexer)?;

        let end: Date = Date::parse(lexer)?;

        Ok(ContactInfo::new(tx_node_id, rx_node_id, start, end))
    }
}
