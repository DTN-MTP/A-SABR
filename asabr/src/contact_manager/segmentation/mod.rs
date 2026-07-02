extern crate alloc;

use alloc::vec::Vec;

use crate::contact::ContactInfo;
use crate::parse_transparent;
use crate::parsing::Parse;
use crate::types::{DataRate, Date, Duration, Volume};

pub mod lex;
pub mod pseg;
pub mod seg;

/// A segment represents a time interval with an associated value of type `T`.
#[derive(Debug)]
pub struct Segment<T> {
    /// The start time of the segment.
    pub start: Date,
    /// The end time of the segment.
    pub end: Date,
    /// The value associated with the time interval, which could represent rate, delay, or any other characteristic.
    pub val: T,
}

pub type SegmentParse<T> = (Date, (Date, T));

impl<T> From<SegmentParse<T>> for Segment<T> {
    fn from(value: SegmentParse<T>) -> Self {
        let (start, (end, val)) = value;
        Segment { start, end, val }
    }
}

parse_transparent!(Segment<Tt>,SegmentParse<Tt>,Tt: Parse);

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
fn get_delays(
    tx_start: Date,
    tx_end: Date,
    delay_intervals: &Vec<Segment<Duration>>,
) -> (Duration, Duration) {
    let mut i = 0;
    let mut start_delay = Duration::MAX;

    for delay_seg in delay_intervals {
        if tx_start <= delay_seg.end {
            start_delay = delay_seg.val;
            break;
        }
        i += 1;
    }

    for delay_seg in &delay_intervals[i..] {
        if tx_end <= delay_seg.end {
            return (start_delay, delay_seg.val);
        }
    }

    (start_delay, Duration::MAX)
}

/// Attempts to initialize segmentation state by validating interval coverage.
///
/// This function verifies that:
/// - `rate_intervals` fully and contiguously cover the contact time window
/// - `delay_intervals` fully and contiguously cover the contact time window
/// - No gaps or overlaps exist in either interval list
/// - `other_intervals` is initially empty
///
/// If validation succeeds, `other_intervals` is initialized with a single
/// segment covering the full contact window using `default` as its value.
///
/// # Arguments
///
/// * `rate_intervals` - Rate segments that must exactly and contiguously span
///   `[info.start, info.end)`.
/// * `delay_intervals` - Delay segments that must exactly and contiguously span
///   `[info.start, info.end)`.
/// * `other_intervals` - Output interval vector to initialize on success.
///   Must be empty on entry.
/// * `default` - Default value assigned to the initialized segment in
///   `other_intervals`.
/// * `info` - Contact information defining the valid time window.
///
/// # Feature Flags
///
/// When the `first_depleted` feature is enabled, `original_volume` is reset
/// and populated with the total transferable volume computed from
/// `rate_intervals`.
///
/// # Returns
///
/// Returns `true` if:
/// - All interval checks pass
/// - Initialization completes successfully
///
/// Returns `false` if:
/// - Any interval list has gaps
/// - Intervals do not exactly match the contact window
/// - `other_intervals` is not empty
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
        *original_volume = 0;
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
/// * `rate_intervals` - The rate segments defining available bandwidth over time.
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
        // We exceeded the capacity of the segment
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
