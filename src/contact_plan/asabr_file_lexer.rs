extern crate alloc;
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::{
    errors::ASABRError,
    parsing::{Lexer, LexerOutput},
};

/// A lexer for tokenizing text from a file.
///
/// The `FileLexer` reads a file line by line, processes tokens (words), and provides them one at a time for parsing.
/// It skips lines starting with `#`, allowing them to be used as comments in the input file.
pub struct FileLexer<'a, T: Iterator<Item = &'a str>> {
    /// Tracks the current line number during lookup operations.
    lookup_current_line: usize,
    /// Tracks the line number from which the current token was consumed.
    current_line: usize,
    /// Tracks the token's position in the current line.
    token_position: usize,
    /// A buffered reader for the input file.
    reader: T,
    /// A stack that stores tokens (words) from the file, in reverse order, for easy consumption.
    buffer_stack: Vec<&'a str>,
}

impl<'a, T: Iterator<Item = &'a str>> FileLexer<'a, T> {
    /// Creates a new `FileLexer` for the specified file.
    ///
    /// # Arguments
    ///
    /// * `filename` - The path to the file to be read.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the `FileLexer` if the file is successfully opened, or an `io::Error` otherwise.
    ///
    /// # Errors
    ///
    /// Will return an `io::Error` if the file cannot be opened.
    pub fn new(content: T) -> Self {
        Self {
            lookup_current_line: 0,
            current_line: 0,
            token_position: 0,
            reader: content,
            buffer_stack: Vec::new(),
        }
    }

    /// Reads the next line of the file and splits it into words, storing them in the buffer stack.
    ///
    /// This function continues reading until it finds a non-empty line that doesn't start with `#`.
    /// The words are stored in reverse order in the `buffer_stack` to facilitate easy pop operations.
    ///
    /// # Returns
    ///
    /// Returns an `io::Result<()>` indicating success or failure in reading the next line.
    fn read_next_line(&mut self) {
        for line in self.reader.by_ref() {
            self.lookup_current_line += 1;

            // Skip lines starting with '#'
            if line.trim_start().starts_with('#') {
                continue;
            }

            // Split the line into words and collect them into a vector in reverse order
            let words: Vec<_> = line.split_whitespace().rev().collect();
            if !words.is_empty() {
                self.buffer_stack.extend(words);
                return;
            }
        }
    }
}

impl<'a, T: Iterator<Item = &'a str>> Lexer for FileLexer<'a, T> {
    /// Consumes and returns the next token (word) from the file.
    ///
    /// If the buffer is empty, it reads the next line of words into the buffer before consuming a token.
    ///
    /// # Returns
    ///
    /// Returns `Ok(LexerOutput::Finished(String))` if a token is successfully consumed,
    /// `Ok(LexerOutput::EOF)` if the end of the file is reached, or `Err` if an error occurs.
    fn consume_next_token(&mut self) -> Result<LexerOutput<String>, ASABRError> {
        if self.buffer_stack.is_empty() {
            self.read_next_line();
        }

        let next_word = self.buffer_stack.pop();
        match next_word {
            Some(word) => {
                if self.current_line != self.lookup_current_line {
                    self.token_position = 0;
                    self.current_line = self.lookup_current_line;
                }
                self.token_position += 1;
                Ok(LexerOutput::Finished(word.to_string()))
            }
            None => Ok(LexerOutput::EOF),
        }
    }

    /// Returns the current position in the file in terms of line number and token position.
    ///
    /// This method provides a string describing the current position for debugging or error reporting purposes.
    ///
    /// # Returns
    ///
    /// A string in the format `"line {current_line}, token {token_position}"`.
    fn get_current_position(&self) -> (usize, usize) {
        (self.current_line, self.token_position)
    }

    /// Looks at the next token without consuming it.
    ///
    /// If the buffer is empty, it reads the next line of words into the buffer before returning the next token.
    ///
    /// # Returns
    ///
    /// Returns `Ok(LexerOutput::Finished(String))` if a token is available,
    /// `Ok(LexerOutput::EOF)` if the end of the file is reached, or `Err` if an error occurs.
    fn lookup(&mut self) -> Result<LexerOutput<String>, ASABRError> {
        if self.buffer_stack.is_empty() {
            self.read_next_line();
        }

        let next_word = self.buffer_stack.last();
        match next_word {
            Some(word) => Ok(LexerOutput::Finished(word.to_string())),
            None => Ok(LexerOutput::EOF),
        }
    }
}
