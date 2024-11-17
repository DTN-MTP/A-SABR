// The Segmented contacts are composites, construct the contact by adding intervals
// Use is_wellformed for sanity check

use std::cmp::max;
use std::collections::HashMap;

use crate::bundle::Bundle;
use crate::contact::ContactInfo;
use crate::parsing::{DispatchParser, Lexer, Parser, ParsingState};
use crate::types::{DataRate, Date, Duration, Token, Volume};

use super::{ContactManager, TxEndHopData};

/// A segment represents a time interval with an associated value of type `T`.
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Segment<T> {
    /// The start time of the segment.
    pub start: Date,
    /// The end time of the segment.
    pub end: Date,
    /// The value associated with the time interval, which could represent rate, delay, or any other characteristic.
    pub val: T,
}

/// Manages contact segments, where each segment may have a distinct data rate and delay.
///
/// The `SegmentationManager` uses different segments to manage free intervals, rate intervals, and delay intervals,
/// which are applied in contact scheduling and transmission simulation.
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct SegmentationManager {
    /// A list of segments representing free intervals available for transmission.
    free_intervals: Vec<Segment<()>>,
    /// A list of segments representing different data rates during contact intervals.
    rate_intervals: Vec<Segment<DataRate>>,
    /// A list of segments representing delay times associated with different intervals.
    delay_intervals: Vec<Segment<Duration>>,
    #[cfg(feature = "first_depleted")]
    /// The total volume at initialization.
    original_volume: Volume,
}

/// Creates a new `SegmentationManager` with specified rate and delay intervals.
///
/// # Arguments
///
/// * `rate_intervals` - A vector of segments representing different data rates during contact intervals.
/// * `delay_intervals` - A vector of segments representing delay times associated with different intervals.
///
/// # Returns
///
/// A new instance of `SegmentationManager`.
impl SegmentationManager {
    pub fn new(
        rate_intervals: Vec<Segment<DataRate>>,
        delay_intervals: Vec<Segment<Duration>>,
    ) -> Self {
        let free_intervals = Vec::new();

        Self {
            free_intervals,
            rate_intervals,
            delay_intervals,
            #[cfg(feature = "first_depleted")]
            original_volume: 0.0,
        }
    }

    /// Determines the delay based on the transmission end time (`tx_end`) and the available delay intervals.
    ///
    /// # Arguments
    ///
    /// * `tx_end` - The calculated transmission end time.
    /// * `delay_intervals` - A vector of segments representing delay intervals.
    ///
    /// # Returns
    ///
    /// The delay value for the corresponding interval, or `Duration::MAX` if no interval applies.
    #[inline(always)]
    fn get_delay(tx_end: Date, delay_intervals: &Vec<Segment<Duration>>) -> Duration {
        for delay_seg in delay_intervals {
            if tx_end > delay_seg.end {
                continue;
            }
            return delay_seg.val;
        }
        Duration::MAX
    }

    /// Calculates the transmission end time based on the current time, the volume to be transmitted, and the deadline.
    ///
    /// # Arguments
    ///
    /// * `at_time` - The current time for scheduling.
    /// * `volume` - The volume to be transmitted.
    /// * `deadline` - The transmission deadline (end of the contact interval).
    ///
    /// # Returns
    ///
    /// Optionally returns the transmission end time `Date` or `None` if the volume cannot be transmitted by the deadline.
    #[inline(always)]
    fn get_tx_end(&self, mut at_time: Date, mut volume: Volume, deadline: Date) -> Option<Date> {
        let mut tx_end = Date::MAX;

        for rate_seg in &self.rate_intervals {
            if rate_seg.end < at_time {
                continue;
            }

            tx_end = at_time + volume / rate_seg.val;

            if tx_end > rate_seg.end {
                volume -= rate_seg.val * (tx_end - at_time);
                at_time = rate_seg.end;
                continue;
            }
            volume = 0.0;
            break;
        }

        if volume > 0.0 || tx_end > deadline {
            return None;
        }
        Some(tx_end)
    }
}

/// Implements the `ContactManager` trait for `SegmentationManager`, providing methods for simulating and scheduling transmissions.
impl ContactManager for SegmentationManager {
    /// Simulates the transmission of a bundle based on the contact data and available free intervals.
    ///
    /// # Arguments
    ///
    /// * `_contact_data` - Reference to the contact information (unused in this implementation).
    /// * `at_time` - The current time for scheduling purposes.
    /// * `bundle` - The bundle to be transmitted.
    ///
    /// # Returns
    ///
    /// Optionally returns `TxEndHopData` with transmission start and end times, or `None` if the bundle can't be transmitted.
    fn dry_run(
        &self,
        _contact_data: &ContactInfo,
        at_time: Date,
        bundle: &Bundle,
    ) -> Option<TxEndHopData> {
        let mut tx_start: Date;

        for free_seg in &self.free_intervals {
            if free_seg.end < at_time {
                continue;
            }
            tx_start = Date::max(free_seg.start, at_time);
            if let Some(tx_end) = self.get_tx_end(tx_start, bundle.size, free_seg.end) {
                let delay = Self::get_delay(tx_end, &self.delay_intervals);
                return Some(TxEndHopData {
                    tx_start,
                    tx_end,
                    delay,
                    expiration: free_seg.end,
                    arrival: tx_end + delay,
                });
            }
        }
        None
    }
    /// Schedule the transmission of a bundle based on the contact data and available free intervals.
    ///
    /// This method shall be called after a dry run ! Implementations might not ensure a clean behavior otherwise.
    ///
    /// # Arguments
    ///
    /// * `_contact_data` - Reference to the contact information (unused in this implementation).
    /// * `at_time` - The current time for scheduling purposes.
    /// * `bundle` - The bundle to be transmitted.
    ///
    /// # Returns
    ///
    /// Optionally returns `TxEndHopData` with transmission start and end times, or `None` if the bundle can't be transmitted.
    fn schedule(
        &mut self,
        _contact_data: &ContactInfo,
        at_time: Date,
        bundle: &Bundle,
    ) -> Option<TxEndHopData> {
        let mut tx_start = 0.0;
        let mut index = 0;
        let mut tx_end = 0.0;

        for free_seg in &self.free_intervals {
            if free_seg.end < at_time {
                continue;
            }
            tx_start = Date::max(free_seg.start, at_time);
            if let Some(tx_end_res) = self.get_tx_end(tx_start, bundle.size, free_seg.end) {
                tx_end = tx_end_res;
                break;
            }
            index += 1;
        }

        let interval = &mut self.free_intervals[index];
        let expiration = interval.end;
        let delay = Self::get_delay(tx_end, &self.delay_intervals);

        if interval.start != tx_start {
            interval.end = tx_start;
            self.free_intervals.insert(
                index + 1,
                Segment {
                    start: tx_end,
                    end: expiration,
                    val: (),
                },
            )
        } else {
            interval.start = tx_end;
        }

        Some(TxEndHopData {
            tx_start,
            tx_end,
            delay,
            expiration,
            arrival: tx_end + delay,
        })
    }

    /// Initializes the segmentation manager by checking that rate and delay intervals have no gaps.
    ///
    /// # Arguments
    ///
    /// * `contact_data` - Reference to the contact information.
    ///
    /// # Returns
    ///
    /// Returns `true` if initialization is successful, or `false` if there are gaps in the intervals.
    fn try_init(&mut self, contact_data: &ContactInfo) -> bool {
        // we check that we have no holes for rate segments
        let mut time = contact_data.start;
        for inter in &self.rate_intervals {
            if inter.start != time {
                return false;
            }
            time = inter.end;
            #[cfg(feature = "first_depleted")]
            {
                self.original_volume += (inter.end - inter.start) * inter.val;
            }
        }
        let opt_rate_end = self.rate_intervals.last();
        match opt_rate_end {
            Some(last_rate_seg) => {
                if last_rate_seg.end != contact_data.end {
                    return false;
                }
            }
            None => return false,
        }

        // we check that we have no holes for delay segments
        time = contact_data.start;
        for inter in &self.delay_intervals {
            if inter.start != time {
                return false;
            }
            time = inter.end;
        }

        let opt_delay_end = self.delay_intervals.last();
        match opt_delay_end {
            Some(last_delay_seg) => {
                if last_delay_seg.end != contact_data.end {
                    return false;
                }
            }

            None => return false,
        }

        // if there are no holes, just create a free segment for the whole contact period
        self.free_intervals.push(Segment {
            start: contact_data.start,
            end: contact_data.end,
            val: (),
        });

        true
    }

    /// For first depleted compatibility
    ///
    /// # Returns
    ///
    /// Returns the maximum volume the contact had at initialization.
    #[cfg(feature = "first_depleted")]
    fn get_original_volume(&self) -> Volume {
        self.original_volume
    }
}

/// Parses an interval, consisting of a start date, end date, and a value of type `T`, from the lexer.
///
/// The interval is expected to have three components in the following order:
/// 1. Start date (`Date`)
/// 2. End date (`Date`)
/// 3. Value of type `T` (e.g., `DataRate`, `Duration`)
///
/// # Arguments
///
/// * `lexer` - A mutable reference to the lexer that will provide the tokens to parse.
///
/// # Type Parameters
///
/// * `T` - The type of the value to be parsed for the interval. It must implement the `FromStr` trait to allow parsing from a string.
///
/// # Returns
///
/// Returns a `ParsingState`:
/// - `Finished((start, end, val))` if the interval is successfully parsed.
/// - `Error(msg)` if there is an error during parsing.
/// - `EOF` if an unexpected end-of-file is encountered during parsing.
fn parse_interval<T: std::str::FromStr>(lexer: &mut dyn Lexer) -> ParsingState<(Date, Date, T)> {
    let start: Date;
    let end: Date;
    let val: T;

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

    let val_state = T::parse(lexer);
    match val_state {
        ParsingState::Finished(value) => val = value,
        ParsingState::Error(msg) => return ParsingState::Error(msg),
        ParsingState::EOF => {
            return ParsingState::Error(format!(
                "Parsing failed ({})",
                lexer.get_current_position()
            ))
        }
    }
    ParsingState::Finished((start, end, val))
}

/// Implements the DispatchParser to allow dynamic parsing.
impl DispatchParser<SegmentationManager> for SegmentationManager {}

/// Implements the `Parser` trait for `SegmentationManager`, allowing the manager to be parsed from a lexer.
impl Parser<SegmentationManager> for SegmentationManager {
    /// Parses a `SegmentationManager` from the lexer, extracting the rate and delay intervals.
    ///
    /// # Arguments
    ///
    /// * `lexer` - The lexer used for parsing tokens.
    /// * `_sub` - An optional map for handling custom parsing logic (unused here).
    ///
    /// # Returns
    ///
    /// Returns a `ParsingState` indicating whether parsing was successful (`Finished`) or encountered an error (`Error`).
    fn parse(lexer: &mut dyn Lexer) -> ParsingState<SegmentationManager> {
        let mut rate_intervals: Vec<Segment<DataRate>> = Vec::new();
        let mut delay_intervals: Vec<Segment<Duration>> = Vec::new();

        loop {
            let res = lexer.lookup();
            match res {
                ParsingState::EOF => break,
                ParsingState::Error(e) => return ParsingState::Error(e),
                ParsingState::Finished(interval_type) => match interval_type.as_str() {
                    "delay" => {
                        lexer.consume_next_token();
                        let state = parse_interval::<Duration>(lexer);
                        match state {
                            ParsingState::Finished((start, end, delay)) => {
                                delay_intervals.push(Segment {
                                    start,
                                    end,
                                    val: delay,
                                });
                            }
                            ParsingState::EOF => {
                                return ParsingState::EOF;
                            }
                            ParsingState::Error(msg) => {
                                return ParsingState::Error(msg);
                            }
                        }
                    }
                    "rate" => {
                        lexer.consume_next_token();
                        let state = parse_interval::<DataRate>(lexer);
                        match state {
                            ParsingState::Finished((start, end, rate)) => {
                                rate_intervals.push(Segment {
                                    start,
                                    end,
                                    val: rate,
                                });
                            }
                            ParsingState::EOF => {
                                return ParsingState::EOF;
                            }
                            ParsingState::Error(msg) => {
                                return ParsingState::Error(msg);
                            }
                        }
                    }
                    _ => {
                        break;
                    }
                },
            }
        }
        ParsingState::Finished(SegmentationManager::new(rate_intervals, delay_intervals))
    }
}
