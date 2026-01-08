#[cfg(feature = "first_depleted")]
use crate::types::Volume;
use crate::{
    bundle::Bundle,
    contact::ContactInfo,
    contact_manager::{
        segmentation::{BaseSegmentationManager, Segment},
        ContactManager, ContactManagerTxData,
    },
    parsing::{DispatchParser, Lexer, Parser, ParsingState},
    types::{DataRate, Date, Duration},
};

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

impl SegmentationManager {
    /// Creates a new [`SegmentationManager`] from the provided rate and delay intervals.
    ///
    /// This constructor initializes the manager with:
    /// - An empty set of `free_intervals`
    /// - The given `rate_intervals`, which define data rates over contact segments
    /// - The given `delay_intervals`, which define propagation or processing delays
    ///
    /// # Arguments
    ///
    /// * `rate_intervals` - Segments describing data rates over time.
    /// * `delay_intervals` - Segments describing delay durations over time.
    ///
    /// # Feature Flags
    ///
    /// When the `first_depleted` feature is enabled, the `original_volume`
    /// field is initialized to `0.0`.
    ///
    /// # Returns
    ///
    /// A fully initialized [`SegmentationManager`].
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
}

impl BaseSegmentationManager for SegmentationManager {
    /// Delegates construction to [`SegmentationManager::new`].
    fn new(
        rate_intervals: Vec<Segment<DataRate>>,
        delay_intervals: Vec<Segment<Duration>>,
    ) -> Self {
        Self::new(rate_intervals, delay_intervals)
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
    /// Optionally returns `ContactManagerTxData` with transmission start and end times, or `None` if the bundle can't be transmitted.
    fn dry_run_tx(
        &self,
        _contact_data: &ContactInfo,
        at_time: Date,
        bundle: &Bundle,
    ) -> Option<ContactManagerTxData> {
        let mut tx_start: Date;

        for free_seg in &self.free_intervals {
            if free_seg.end < at_time {
                continue;
            }
            tx_start = Date::max(free_seg.start, at_time);
            let Some(tx_end) =
                super::get_tx_end(&self.rate_intervals, tx_start, bundle.size, free_seg.end)
            else {
                continue;
            };

            let delay = super::get_delay(tx_end, &self.delay_intervals);
            return Some(ContactManagerTxData {
                tx_start,
                tx_end,
                delay,
                expiration: free_seg.end,
                arrival: tx_end + delay,
            });
        }
        None
    }

    /// Schedule the transmission of a bundle based on the contact data intervals booked with lower priority load.
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
    /// Optionally returns `ContactManagerTxData` with transmission start and end times, or `None` if the bundle can't be transmitted.
    fn schedule_tx(
        &mut self,
        _contact_data: &ContactInfo,
        at_time: Date,
        bundle: &Bundle,
    ) -> Option<ContactManagerTxData> {
        let mut tx_start = 0.0;
        let mut index = 0;
        let mut tx_end = 0.0;

        for free_seg in &self.free_intervals {
            if free_seg.end < at_time {
                continue;
            }
            tx_start = Date::max(free_seg.start, at_time);
            if let Some(tx_end_res) =
                super::get_tx_end(&self.rate_intervals, tx_start, bundle.size, free_seg.end)
            {
                tx_end = tx_end_res;
                break;
            }
            index += 1;
        }

        let interval = &mut self.free_intervals[index];
        let expiration = interval.end;
        let delay = super::get_delay(tx_end, &self.delay_intervals);

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

        Some(ContactManagerTxData {
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
        super::try_init(
            &self.rate_intervals,
            &self.delay_intervals,
            &mut self.free_intervals,
            (),
            #[cfg(feature = "first_depleted")]
            &mut self.original_volume,
            contact_data,
        )
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
        super::parse::<SegmentationManager>(lexer)
    }
}
