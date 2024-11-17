use crate::contact_manager::ContactManager;
use crate::parsing::{Lexer, Parser, ParsingState};
#[cfg(feature = "contact_work_area")]
use crate::route_stage::RouteStage;
use crate::types::{Date, NodeID, Token};
#[cfg(feature = "contact_work_area")]
use std::cell::RefCell;
use std::cmp::Ordering;
#[cfg(feature = "contact_work_area")]
use std::rc::Rc;

/// Represents basic information about a contact between two nodes.
#[derive(Clone, Copy)]
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct ContactInfo {
    ///The ID of the transmitting node.
    tx_node: NodeID,
    /// The ID of the receiving node.
    rx_node: NodeID,
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
    /// * `tx_node` - The ID of the transmitting node.
    /// * `rx_node` - The ID of the receiving node.
    /// * `start` - The start time of the contact.
    /// * `end` - The end time of the contact.
    ///
    /// # Returns
    ///
    /// * `Self` - A new instance of `ContactInfo`.
    pub fn new(tx_node: NodeID, rx_node: NodeID, start: Date, end: Date) -> Self {
        Self {
            tx_node,
            rx_node,
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
/// - `CM`: A type implementing the `ContactManager` trait, responsible for managing the
///   contact's operations.
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Contact<CM: ContactManager> {
    /// The basic information about the contact.
    pub info: ContactInfo,
    /// The manager handling the contact's operations.
    pub manager: CM,
    #[cfg(feature = "contact_work_area")]
    /// The work area for managing path construction stages (compilation option).
    pub work_area: Rc<RefCell<RouteStage<CM>>>,
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
    pub fn try_new(info: ContactInfo, mut manager: CM) -> Option<Self> {
        if info.try_init() && manager.try_init(&info) {
            #[cfg(feature = "contact_work_area")]
            let to_node = info.rx_node;
            return Some(Contact {
                info,
                manager,
                #[cfg(feature = "contact_work_area")]
                work_area: Rc::new(RefCell::new(RouteStage::new_work_area(to_node))),
                #[cfg(feature = "contact_suppression")]
                suppressed: false,
            });
        }
        None
    }

    /// Retrieves the transmitting node's ID.
    ///
    /// # Returns
    ///
    /// * `NodeID` - The ID of the transmitting node.
    pub fn get_tx_node(&self) -> NodeID {
        self.info.tx_node
    }

    /// Retrieves the receiving node's ID.
    ///
    /// # Returns
    ///
    /// * `NodeID` - The ID of the receiving node.
    pub fn get_rx_node(&self) -> NodeID {
        self.info.rx_node
    }
}

impl<CM: ContactManager> Ord for Contact<CM> {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.info.tx_node > other.info.tx_node {
            return Ordering::Greater;
        }
        if self.info.tx_node < other.info.tx_node {
            return Ordering::Less;
        }
        if self.info.rx_node > other.info.rx_node {
            return Ordering::Greater;
        }
        if self.info.rx_node < other.info.rx_node {
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

impl<CM: ContactManager> PartialOrd for Contact<CM> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<CM: ContactManager> PartialEq for Contact<CM> {
    fn eq(&self, other: &Self) -> bool {
        self.info.tx_node == other.info.tx_node
            && self.info.rx_node == other.info.rx_node
            && self.info.start < other.info.start
    }
}
impl<CM: ContactManager> Eq for Contact<CM> {}

impl Parser<ContactInfo> for ContactInfo {
    /// Parses a `ContactInfo` from a lexer.
    ///
    /// # Parameters
    ///
    /// * `lexer` - A mutable reference to a lexer that provides tokens for parsing.
    ///
    /// # Returns
    ///
    /// * `ParsingState<ContactInfo>` - The parsing state indicating success or failure.
    fn parse(lexer: &mut dyn Lexer) -> ParsingState<ContactInfo> {
        let tx_node: NodeID;
        let rx_node: NodeID;
        let start: Date;
        let end: Date;

        let tx_node_state = NodeID::parse(lexer);
        match tx_node_state {
            ParsingState::Finished(value) => tx_node = value,
            ParsingState::Error(msg) => return ParsingState::Error(msg),
            ParsingState::EOF => {
                return ParsingState::Error(format!(
                    "Parsing failed ({})",
                    lexer.get_current_position()
                ))
            }
        }

        let rx_node_state = NodeID::parse(lexer);
        match rx_node_state {
            ParsingState::Finished(value) => rx_node = value,
            ParsingState::Error(msg) => return ParsingState::Error(msg),
            ParsingState::EOF => {
                return ParsingState::Error(format!(
                    "Parsing failed ({})",
                    lexer.get_current_position()
                ))
            }
        }

        let start_state = Date::parse(lexer);
        match start_state {
            ParsingState::Finished(value) => start = value,
            ParsingState::Error(msg) => return ParsingState::Error(msg),
            ParsingState::EOF => {
                return ParsingState::Error(format!(
                    "Parsing failed ({})",
                    lexer.get_current_position()
                ))
            }
        }

        let end_state = Date::parse(lexer);
        match end_state {
            ParsingState::Finished(value) => end = value,
            ParsingState::Error(msg) => return ParsingState::Error(msg),
            ParsingState::EOF => {
                return ParsingState::Error(format!(
                    "Parsing failed ({})",
                    lexer.get_current_position()
                ))
            }
        }

        ParsingState::Finished(ContactInfo::new(tx_node, rx_node, start, end))
    }
}
