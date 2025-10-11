use std::collections::HashMap;

use crate::{contact_manager::ContactManager, node_manager::NodeManager};

pub type ContactMarkerMap<'a> = Dispatcher<'a, ContactDispatcher>;
pub type NodeMarkerMap<'a> = Dispatcher<'a, NodeDispatcher>;
pub type ContactDispatcher = fn(&mut dyn Lexer) -> ParsingState<Box<dyn ContactManager>>;
pub type NodeDispatcher = fn(&mut dyn Lexer) -> ParsingState<Box<dyn NodeManager>>;

/// Wrapper object to a marker -> coercion function map for contacts or nodes versions (T)
///
/// # Type Parameters
///
/// * `T`: The function type of the values being stored, use ContactDispatcher or NodeDispatcher.
pub struct Dispatcher<'a, T> {
    /// A hashmap that stores the coercion functions with their associated markers.
    map: HashMap<&'a str, T>,
}
impl<'a, T> Dispatcher<'a, T> {
    /// Creates a new, empty `Dispatcher`.
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Adds a new entry to the dispatcher.
    ///
    /// # Arguments
    ///
    /// * `marker` - A string slice that acts as the unique key for `coerce_fn`.
    /// * `coerce_fn` - The function of type `T` to associate with `marker`.
    pub fn add(&mut self, marker: &'a str, coerce_fn: T) {
        self.map.insert(marker, coerce_fn);
    }

    /// Retrieves the coercion function associated with the given `marker`, if it exists.
    ///
    /// # Arguments
    ///
    /// * `marker` - A string slice representing the key for the desired value.
    ///
    /// # Returns
    ///
    /// An `Option` containing a reference to the value of type `T` if it exists, or `None` if
    /// the `marker` is not found.
    pub fn get(&self, marker: &'a str) -> Option<&T> {
        return self.map.get(marker);
    }
}

/// Represents the state of parsing for a generic type.
pub enum ParsingState<T> {
    /// Indicates that the end of the file has been reached.
    EOF,
    /// Contains an error message indicating what went wrong during parsing.
    Error(String),
    /// Contains the successfully parsed value of type `T`.
    Finished(T),
}

/// Trait for a lexer that reads input and returns parsed tokens.
pub trait Lexer {
    /// Looks up the next token in the input stream.
    fn lookup(&mut self) -> ParsingState<String>;
    /// Consumes and returns the next token from the input stream.
    fn consume_next_token(&mut self) -> ParsingState<String>;
    /// Returns the current position in the input stream.
    fn get_current_position(&self) -> String;
}

/// Trait for parsing a generic type `T` from a lexer.
pub trait Parser<T> {
    ///  Parses an instance of type `T` from the provided lexer.
    fn parse(lexer: &mut dyn Lexer) -> ParsingState<T>;
}

/// Delegate the parsing logic to the boxed Parser type.
impl<T: Parser<T>> Parser<Box<T>> for Box<T> {
    ///  Parses an instance of type `T` an return a boxed type.
    fn parse(lexer: &mut dyn Lexer) -> ParsingState<Box<T>> {
        let ret = T::parse(lexer);
        match ret {
            ParsingState::EOF => ParsingState::EOF,
            ParsingState::Error(msg) => ParsingState::Error(msg),
            ParsingState::Finished(val) => ParsingState::Finished(Box::new(val)),
        }
    }
}

/// Macro to generate the unability to dispatch for concrete Parser types.
macro_rules! implement_parser {
    ($manager_type:ident) => {
        /// Dispatching is impossible for concrete Parser types.
        impl Parser<Box<dyn $manager_type>> for Box<dyn $manager_type> {
            fn parse(_lexer: &mut dyn Lexer) -> ParsingState<Box<dyn $manager_type>> {
                panic!("Unable to dispatch to the correct parser, the Dispatcher");
            }
        }
    };
}

implement_parser!(NodeManager);
implement_parser!(ContactManager);

/// Parses compgitonents including info and manager from the lexer.
///
/// # Parameters
///
/// * `lexer` - The lexer used to read the components.
/// * `dispatch_map` - An optional map for dispatching manager parsing functions.
///
/// # Returns
///
/// * `ParsingState<(INFO, MANAGER)>` - The parsing state containing either the parsed components or an error.
pub fn parse_components<INFO: Parser<INFO>, MANAGER: DispatchParser<MANAGER> + Parser<MANAGER>>(
    lexer: &mut dyn Lexer,
    dispatch_map: Option<&Dispatcher<fn(&mut dyn Lexer) -> ParsingState<MANAGER>>>,
) -> ParsingState<(INFO, MANAGER)> {
    let info: INFO;
    let manager: MANAGER;

    let info_state = INFO::parse(lexer);
    match info_state {
        ParsingState::Finished(value) => info = value,
        ParsingState::Error(msg) => return ParsingState::Error(msg),
        ParsingState::EOF => {
            return ParsingState::Error(format!(
                "Parsing failed ({})",
                lexer.get_current_position()
            ))
        }
    }

    let manager_state = MANAGER::parse_dispatch(lexer, dispatch_map);
    match manager_state {
        ParsingState::Finished(value) => manager = value,
        ParsingState::Error(msg) => return ParsingState::Error(msg),
        ParsingState::EOF => {
            return ParsingState::Error(format!(
                "Parsing failed ({})",
                lexer.get_current_position()
            ))
        }
    }
    ParsingState::Finished((info, manager))
}

/// Trait for parsing a manager type `T` from a lexer.
///
/// # Methods
///
/// * `parse` - Parses an instance of type `T` from the provided lexer and optional marker map.
pub trait DispatchParser<T: Parser<T>> {
    /// Parses a manager component from the lexer.
    ///
    /// # Parameters
    ///
    /// * `lexer` - A mutable reference to the lexer that is used to read the manager component.
    ///     - **Type**: `&mut dyn Lexer`
    /// * `marker_map` - An optional map of markers to functions that can parse specific types of managers.
    ///     - **Type**: `Option<&Dispatcher<fn(&mut dyn Lexer) -> ParsingState<T>>>`
    ///     - This map allows dynamic dispatch of the parsing functions based on the markers found in the input.
    ///
    /// # Returns
    ///
    /// * `ParsingState<T>` - The parsing state which can be:
    ///     - `Finished(T)` - Indicates successful parsing with the parsed manager component.
    ///     - `Error(String)` - Indicates an error encountered during parsing with an error message.
    ///     - `EOF` - Indicates the end of the input stream, suggesting that parsing cannot continue.
    fn parse_dispatch(
        lexer: &mut dyn Lexer,
        _marker_map: Option<&Dispatcher<fn(&mut dyn Lexer) -> ParsingState<T>>>,
    ) -> ParsingState<T> {
        T::parse(lexer)
    }
}

// Implement DispatchParser for Box<T>.
impl<T: DispatchParser<T> + Parser<T>> DispatchParser<Box<T>> for Box<T> {
    /// Delegates the parsing to the Parser trait.
    fn parse_dispatch(
        lexer: &mut dyn Lexer,
        _: Option<&Dispatcher<fn(&mut dyn Lexer) -> ParsingState<Box<T>>>>,
    ) -> ParsingState<Box<T>> {
        <Box<T>>::parse(lexer)
    }
}

/// Macro to implement parsing functionality.
///
/// # Parameters
///
/// * `$manager_type` - The type of the manager to implement parsing for.
/// * `$coerce_fn` - The name of the coercion function to generate.
macro_rules! implement_manager {
    ($manager_type:ident, $coerce_fn:ident) => {
        /// Forces parsing to a concrete type and returns the boxed value as a boxed dynamic type.
        pub fn $coerce_fn<'a, M>(lexer: &mut dyn Lexer) -> ParsingState<Box<dyn $manager_type + 'a>>
        where
            M: $manager_type + Parser<M> + 'a,
        {
            let ret = M::parse(lexer);
            match ret {
                ParsingState::EOF => ParsingState::EOF,
                ParsingState::Error(msg) => ParsingState::Error(msg),
                ParsingState::Finished(val) => {
                    ParsingState::Finished(Box::new(val) as Box<dyn $manager_type + 'a>)
                }
            }
        }

        /// Delegates the parsing to the correct Parser concrete implementation after dispatching.
        impl DispatchParser<Box<dyn $manager_type>> for Box<dyn $manager_type> {
            /// Used the marker map to delegate/dispatch the parsing logic to a coercion function.
            fn parse_dispatch(
                lexer: &mut dyn Lexer,
                marker_map_opt: Option<
                    &Dispatcher<fn(&mut dyn Lexer) -> ParsingState<Box<dyn $manager_type>>>,
                >,
            ) -> ParsingState<Box<dyn $manager_type>> {
                let res = lexer.consume_next_token();
                match res {
                    ParsingState::EOF => ParsingState::EOF,
                    ParsingState::Error(msg) => ParsingState::Error(msg),
                    ParsingState::Finished(marker) => {
                        if let Some(marker_map) = marker_map_opt {
                            if let Some(parse_fn) = marker_map.get(marker.as_str()) {
                                parse_fn(lexer)
                            } else {
                                ParsingState::Error(format!(
                                    "Unrecognized marker ({})",
                                    lexer.get_current_position()
                                ))
                            }
                        } else {
                            ParsingState::Error(format!(
                                "Dynamic parsing requires a map ({})",
                                lexer.get_current_position()
                            ))
                        }
                    }
                }
            }
        }
    };
}

// Generate implementations for VolumeManager and NodeManager
implement_manager!(ContactManager, coerce_cm);
implement_manager!(NodeManager, coerce_nm);
