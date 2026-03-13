use std::{collections::HashMap, str::FromStr};

use crate::parsing::{Lexer, ParsingState};

// Convenient for vector indexing
// TODO: add a check like ~ static_assert(sizeof(NodeID) <= sizeof(usize))

/// Represents the unique inner identifier for a node.
pub type NodeID = u16;

/// Represents an element in the chain of NodeIDs that ends with a vnode name.
pub enum VirtualNodeElement {
    NodeID(NodeID),
    /// This variant starts the VirtualNodeElement chain.
    StartDelimiter(String),
    /// This variant ends the VirtualNodeElement chain.
    EndDelimiter(String),
}

impl FromStr for VirtualNodeElement {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(id) = s.parse::<NodeID>() {
            return Ok(VirtualNodeElement::NodeID(id));
        }
        if s == "[" {
            return Ok(VirtualNodeElement::StartDelimiter(s.to_string()));
        }
        if s == "]" {
            return Ok(VirtualNodeElement::EndDelimiter(s.to_string()));
        }

        Err("Error while parsing VirtualNodeElement".into())
    }
}

/// Represents a HashMap with virtual node IDs as keys and real node ID lists as values
pub type VirtualNodeMap = HashMap<NodeID, Vec<NodeID>>;

/// Represents the name of a node.
pub type NodeName = String;

/// Represents a duration in units (e.g., seconds).
pub type Duration = f64;

/// Represents a date (could represent days since a specific epoch).
pub type Date = f64;

/// Represents the priority of a task or node.
pub type Priority = i8;

/// Represents the volume of data (in bytes, for example).
pub type Volume = f64;

/// Represents a data transfer rate (in bits per second).
pub type DataRate = f64;

/// Represents the count of hops in a routing path.
pub type HopCount = u16;

/// A trait for types that can be parsed from a lexer.
///
/// # Type Parameters
///
/// * `T` - The type that will be parsed from the lexer.
pub trait Token<T: Sized> {
    /// Parses a token from the lexer.
    ///
    /// # Parameters
    ///
    /// * `lexer` - A mutable reference to the lexer that provides the token.
    ///
    /// # Returns
    ///
    /// A `ParsingState<T>` indicating the result of the parsing operation.
    fn parse(lexer: &mut dyn Lexer) -> ParsingState<T>;
}

/// Implement the `Token` trait for any type that implements `FromStr`.
impl<T: FromStr> Token<T> for T {
    fn parse(lexer: &mut dyn Lexer) -> ParsingState<T> {
        let res = lexer.consume_next_token();
        match res {
            ParsingState::EOF => ParsingState::EOF,
            ParsingState::Error(e) => ParsingState::Error(e),
            ParsingState::Finished(token) => match token.parse::<T>() {
                Ok(value) => ParsingState::Finished(value),
                Err(_) => ParsingState::Error(format!(
                    "Parsing failed ({})",
                    lexer.get_current_position()
                )),
            },
        }
    }
}

pub fn parse_vec<T: FromStr>(lexer: &mut dyn Lexer) -> ParsingState<Vec<T>> {
    let mut items = Vec::new();
    let mut started = false;

    let try_push = |s: &str, items: &mut Vec<T>| -> bool {
        if s.is_empty() {
            return true;
        }
        match s.parse::<T>() {
            Ok(v) => {
                items.push(v);
                true
            }
            Err(_) => false,
        }
    };

    loop {
        let token = match lexer.consume_next_token() {
            ParsingState::Finished(t) => t,
            ParsingState::EOF => return ParsingState::EOF,
            ParsingState::Error(e) => return ParsingState::Error(e),
        };
        let token = token.trim();

        if !started {
            if !token.starts_with('[') {
                return ParsingState::Error(format!(
                    "Parsing failed, expected '[' ({})",
                    lexer.get_current_position()
                ));
            }
            started = true;
        }

        let token = token.trim_start_matches('[').trim();
        let closes = token.ends_with(']');
        let inner = if closes {
            token.trim_end_matches(']').trim()
        } else {
            token
        };

        if !try_push(inner, &mut items) {
            return ParsingState::Error(format!(
                "Parsing failed ({})",
                lexer.get_current_position()
            ));
        }

        if closes {
            return ParsingState::Finished(items);
        }
    }
}
