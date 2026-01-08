use crate::contact::ContactInfo;
use crate::parsing::{Lexer, ParsingState};
use crate::types::{DataRate, Date, Duration, Token, Volume};

pub mod pseg;
pub mod seg;

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

/// Initializes a segmentation manager by checking that rate and delay intervals have no gaps.
/// Initializes specific values per implementation
///
/// # Arguments
///
/// * `contact_data` - Reference to the contact information.
///
/// # Returns
///
/// Returns `true` if initialization is successful, or `false` if there are gaps in the intervals.
fn try_init<T>(
    rate_intervals: &Vec<Segment<DataRate>>,
    delay_intervals: &Vec<Segment<Duration>>,
    other_intervals: &mut Vec<Segment<T>>,
    default: T,
    #[cfg(feature = "first_depleted")] original_volume: &mut Volume,
    info: &ContactInfo,
) -> bool {
    // we check that we have no holes for rate segments
    let mut time = info.start;
    #[cfg(feature = "first_depleted")]
    {
        *original_volume = 0.0;
    }

    for inter in rate_intervals {
        if inter.start != time {
            return false;
        }
        time = inter.end;
        #[cfg(feature = "first_depleted")]
        {
            *original_volume += (inter.end - inter.start) * inter.val;
        }
    }
    let opt_rate_end = rate_intervals.last();
    match opt_rate_end {
        Some(last_rate_seg) => {
            if last_rate_seg.end != info.end {
                return false;
            }
        }
        None => return false,
    }

    // we check that we have no holes for delay segments
    time = info.start;
    for inter in delay_intervals {
        if inter.start != time {
            return false;
        }
        time = inter.end;
    }

    let opt_delay_end = delay_intervals.last();
    match opt_delay_end {
        Some(last_delay_seg) => {
            if last_delay_seg.end != info.end {
                return false;
            }
        }

        None => return false,
    }

    if !other_intervals.is_empty() {
        return false;
    }

    other_intervals.push(Segment {
        start: info.start,
        end: info.end,
        val: default,
    });

    true
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
fn get_tx_end(
    rate_intervals: &Vec<Segment<DataRate>>,
    mut at_time: Date,
    mut volume: Volume,
    deadline: Date,
) -> Option<Date> {
    for rate_seg in rate_intervals {
        if rate_seg.end < at_time {
            continue;
        }

        // try to get the volume from this segment
        let tx_end = at_time + volume / rate_seg.val;
        // do not exceed deadline (e.g. current available segment)
        if tx_end > deadline {
            return None;
        }
        // We exceeded the capacity of the segement
        if tx_end > rate_seg.end {
            // take everything by updating the remaining volume
            volume -= rate_seg.val * (rate_seg.end - at_time);
            // update at time for next segment
            at_time = rate_seg.end;
            continue;
        }
        // transmission completed on this segment
        return Some(tx_end);
    }
    // some volume was not transmitted
    None
}

/// Common constructor interface for segmentation managers.
///
/// This trait allows different segmentation manager implementations
/// to be instantiated in a uniform way.
pub trait BaseSegmentationManager {
    /// Creates a new segmentation manager from rate and delay intervals.
    /// Required for generic construction for generic parsing logic.
    ///
    /// Implementations are expected to preserve the semantic meaning
    /// of the provided intervals and perform any required internal
    /// initialization.
    ///
    /// # Arguments
    ///
    /// * `rate_intervals` - Segments describing data rates over time.
    /// * `delay_intervals` - Segments describing delay durations over time.
    ///
    /// # Returns
    ///
    /// A new instance of the implementing type.
    fn new(rate_intervals: Vec<Segment<DataRate>>, delay_intervals: Vec<Segment<Duration>>)
        -> Self;
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

/// Parses a `BaseSegmentationManager` from the lexer, extracting the rate and delay intervals.
///
/// # Arguments
///
/// * `lexer` - The lexer used for parsing tokens.
/// * `_sub` - An optional map for handling custom parsing logic (unused here).
///
/// # Returns
///
/// Returns a `ParsingState` indicating whether parsing was successful (`Finished`) or encountered an error (`Error`).
fn parse<M: BaseSegmentationManager>(lexer: &mut dyn Lexer) -> ParsingState<M> {
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
    ParsingState::Finished(M::new(rate_intervals, delay_intervals))
}
