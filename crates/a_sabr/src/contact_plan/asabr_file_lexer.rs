use std::{
    fs::File,
    io::{self, BufRead, BufReader},
};

use crate::parsing::{Lexer, ParsingState};

/// A lexer for tokenizing text from a file.
///
/// The `FileLexer` reads a file line by line, processes tokens (words), and provides them one at a time for parsing.
/// It skips lines starting with `#`, allowing them to be used as comments in the input file.
pub struct FileLexer {
    /// Tracks the current line number during lookup operations.
    lookup_current_line: u32,
    /// Tracks the line number from which the current token was consumed.
    current_line: u32,
    /// Tracks the token's position in the current line.
    token_position: u32,
    /// A buffered reader for the input file.
    reader: BufReader<File>,
    /// A stack that stores tokens (words) from the file, in reverse order, for easy consumption.
    buffer_stack: Vec<String>,
}

impl FileLexer {
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
    pub fn new(filename: &str) -> io::Result<Self> {
        let file = File::open(filename)?;
        let reader = BufReader::new(file);
        Ok(Self {
            lookup_current_line: 0,
            current_line: 0,
            token_position: 0,
            reader,
            buffer_stack: Vec::new(),
        })
    }

    /// Reads the next line of the file and splits it into words, storing them in the buffer stack.
    ///
    /// This function continues reading until it finds a non-empty line that doesn't start with `#`.
    /// The words are stored in reverse order in the `buffer_stack` to facilitate easy pop operations.
    ///
    /// # Returns
    ///
    /// Returns an `io::Result<()>` indicating success or failure in reading the next line.
    fn read_next_words(&mut self) -> io::Result<()> {
        loop {
            let mut line = String::new();
            let bytes_read = self.reader.read_line(&mut line)?;

            if bytes_read == 0 {
                return Ok(());
            }

            self.lookup_current_line += 1;

            // Skip lines starting with '#'
            if line.trim_start().starts_with('#') {
                continue;
            }

            // Split the line into words and collect them into a vector in reverse order
            let words: Vec<String> = line.split_whitespace().rev().map(String::from).collect();
            if words.is_empty() {
                continue;
            }

            self.buffer_stack.extend(words);
            return Ok(());
        }
    }
}

impl Lexer for FileLexer {
    /// Consumes and returns the next token (word) from the file.
    ///
    /// If the buffer is empty, it reads the next line of words into the buffer before consuming a token.
    ///
    /// # Returns
    ///
    /// Returns `ParsingState::Finished(String)` if a token is successfully consumed,
    /// `ParsingState::EOF` if the end of the file is reached, or `ParsingState::Error` if an error occurs.
    fn consume_next_token(&mut self) -> ParsingState<String> {
        if self.buffer_stack.is_empty() {
            let res = self.read_next_words();
            match res {
                Ok(_) => {}
                Err(e) => return ParsingState::Error(e.to_string()),
            }
        }

        let next_word = self.buffer_stack.pop();
        match next_word {
            Some(word) => {
                if self.current_line != self.lookup_current_line {
                    self.token_position = 0;
                    self.current_line = self.lookup_current_line;
                }
                self.token_position += 1;
                ParsingState::Finished(word)
            }
            None => ParsingState::EOF,
        }
    }

    /// Returns the current position in the file in terms of line number and token position.
    ///
    /// This method provides a string describing the current position for debugging or error reporting purposes.
    ///
    /// # Returns
    ///
    /// A string in the format `"line {current_line}, token {token_position}"`.
    fn get_current_position(&self) -> String {
        format!("line {}, token {}", self.current_line, self.token_position)
    }

    /// Looks at the next token without consuming it.
    ///
    /// If the buffer is empty, it reads the next line of words into the buffer before returning the next token.
    ///
    /// # Returns
    ///
    /// Returns `ParsingState::Finished(String)` if a token is available,
    /// `ParsingState::EOF` if the end of the file is reached, or `ParsingState::Error` if an error occurs.
    fn lookup(&mut self) -> ParsingState<String> {
        if self.buffer_stack.is_empty() {
            let res = self.read_next_words();
            match res {
                Ok(_) => {}
                Err(e) => return ParsingState::Error(e.to_string()),
            }
        }

        let next_word = self.buffer_stack.last();
        match next_word {
            Some(word) => ParsingState::Finished(word.to_string()),
            None => ParsingState::EOF,
        }
    }
}
