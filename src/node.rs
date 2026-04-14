use std::{cell::RefCell, cmp::Ordering, rc::Rc};

use crate::{
    errors::ASABRError,
    node_manager::NodeManager,
    parsing::{Lexer, Parser},
    types::{NodeID, NodeName, Token},
};

/// Represents information about a node in the network.
///
/// # Fields
///
/// * `id` - The unique identifier for the node.
/// * `name` - The name associated with the node.
/// * `excluded` - Indicates whether the node is excluded from routing operations.
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

pub type SharedNode<NM> = Rc<RefCell<Node<NM>>>;

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
    /// * `Result<LexerOutput<NodeInfo>, ASABRError>` - The parsing state, which can be either finished with the parsed node info,
    ///   an error, or an EOF state.
    fn parse(lexer: &mut dyn Lexer) -> Result<NodeInfo, ASABRError> {
        let id: NodeID = NodeID::parse(lexer)?;

        let name: NodeName = NodeName::parse(lexer)?;

        Ok(NodeInfo {
            id,
            name,
            excluded: false,
        })
    }
}
