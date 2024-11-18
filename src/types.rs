use std::str::FromStr;

use crate::parsing::{Lexer, ParsingState};

// Convenient for vector indexing
// TODO: add a check like ~ static_assert(sizeof(NodeID) <= sizeof(usize))

/// Represents the unique inner identifier for a node.
pub type NodeID = u16;

/// Represents the name of a node.
pub type NodeName = String;

/// Represents a duration in units (e.g., seconds).
pub type Duration = f64;

/// Represents a date (could represent days since a specific epoch).
pub type Date = f64;

/// Represents the priority of a task or node.
pub type Priority = u8;

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
pub trait Token<T> {
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

impl<T: FromStr> Token<T> for T {
    /// Implement the `Token` trait for any type that implements `FromStr`.
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
