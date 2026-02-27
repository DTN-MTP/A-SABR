use std::cmp::Ordering;

use crate::{
    node_manager::NodeManager,
    parsing::{Lexer, Parser, ParsingState},
    types::{NodeID, NodeName, Token},
};

/// Represents information about a node in the network.
///
/// # Fields
///
/// * `id` - The unique identifier for the node.
/// * `name` - The name associated with the node.
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct NodeInfo {
    pub id: NodeID,
    pub name: NodeName,
    pub excluded: bool,
}

/// Represents a node in the network, including its information and associated manager.
///
/// # Type parameters
/// - `NM`: A type implementing the `NodeManager` trait, responsible for managing the
///   node's operations.
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Node<NM: NodeManager> {
    /// The information about the node, including its ID and name.
    pub info: NodeInfo,
    /// The manager responsible for handling the node's operations.
    pub manager: NM,
}

impl<NM: NodeManager> Node<NM> {
    /// Tries to create a new instance of `Node`.
    ///
    /// # Parameters
    ///
    /// * `info` - The information about the node.
    /// * `manager` - The manager responsible for handling the node's operations.
    ///
    /// # Returns
    ///
    /// * `Option<Self>` - An `Option` containing the new node if successful, or `None`.
    pub fn try_new(info: NodeInfo, manager: NM) -> Option<Self> {
        Some(Node { info, manager })
    }

    /// Retrieves the ID of the node.
    ///
    /// # Returns
    ///
    /// * `NodeID` - The unique identifier of the node.
    pub fn get_node_id(&self) -> NodeID {
        self.info.id
    }

    /// Retrieves the name of the node.
    ///
    /// # Returns
    ///
    /// * `NodeName` - The name of the node.
    pub fn get_node_name(&self) -> NodeName {
        self.info.name.clone()
    }
}

impl<NM: NodeManager> Ord for Node<NM> {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.info.id > other.info.id {
            return Ordering::Greater;
        }
        if self.info.id < other.info.id {
            return Ordering::Less;
        }
        Ordering::Equal
    }
}

impl<NM: NodeManager> PartialOrd for Node<NM> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<NM: NodeManager> PartialEq for Node<NM> {
    fn eq(&self, other: &Self) -> bool {
        self.info.id == other.info.id
    }
}
impl<NM: NodeManager> Eq for Node<NM> {}

impl Parser<NodeInfo> for NodeInfo {
    /// Parses a `NodeInfo` from the provided lexer.
    ///
    /// # Parameters
    ///
    /// * `lexer` - The lexer used to read the node information.
    ///
    /// # Returns
    ///
    /// * `ParsingState<NodeInfo>` - The parsing state, which can be either finished with the parsed node info,
    ///   an error, or an EOF state.
    fn parse(lexer: &mut dyn Lexer) -> ParsingState<NodeInfo> {
        let id: NodeID;
        let name: NodeName;

        let id_state = NodeID::parse(lexer);
        match id_state {
            ParsingState::Finished(value) => id = value,
            ParsingState::Error(msg) => return ParsingState::Error(msg),
            ParsingState::EOF => {
                return ParsingState::Error(format!(
                    "Parsing failed ({})",
                    lexer.get_current_position()
                ));
            }
        }

        let name_state = NodeName::parse(lexer);
        match name_state {
            ParsingState::Finished(value) => name = value,
            ParsingState::Error(msg) => return ParsingState::Error(msg),
            ParsingState::EOF => {
                return ParsingState::Error(format!(
                    "Parsing failed ({})",
                    lexer.get_current_position()
                ));
            }
        }
        ParsingState::Finished(NodeInfo {
            id,
            name,
            excluded: false,
        })
    }
}
