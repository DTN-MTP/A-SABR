#[cfg(feature = "first_depleted")]
use crate::types::Volume;
use crate::{
    bundle::Bundle,
    contact::ContactInfo,
    contact_manager::{
        segmentation::{BaseSegmentationManager, Segment},
        ContactManager, ContactManagerTxData,
    },
    parsing::{Lexer, Parser, ParsingState},
    types::{DataRate, Date, Duration, Priority},
};

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct PSegmentationManager {
    /// A list of segments representing free intervals available for transmission.
    booking: Vec<Segment<Priority>>,
    /// A list of segments representing different data rates during contact intervals.
    rate_intervals: Vec<Segment<DataRate>>,
    /// A list of segments representing delay times associated with different intervals.
    delay_intervals: Vec<Segment<Duration>>,
    #[cfg(feature = "first_depleted")]
    /// The total volume at initialization.
    original_volume: Volume,
}

impl PSegmentationManager {
    pub fn new(
        rate_intervals: Vec<Segment<DataRate>>,
        delay_intervals: Vec<Segment<Duration>>,
    ) -> Self {
        let booking = Vec::new();

        Self {
            booking,
            rate_intervals,
            delay_intervals,
            #[cfg(feature = "first_depleted")]
            original_volume: 0.0,
        }
    }
}

impl BaseSegmentationManager for PSegmentationManager {
    fn new(
        rate_intervals: Vec<Segment<DataRate>>,
        delay_intervals: Vec<Segment<Duration>>,
    ) -> Self {
        Self::new(rate_intervals, delay_intervals)
    }
}

impl ContactManager for PSegmentationManager {
    fn dry_run_tx(
        &self,
        contact_data: &ContactInfo,
        at_time: Date,
        bundle: &Bundle,
    ) -> Option<ContactManagerTxData> {
        let mut tx_start = at_time;
        let mut tx_end_opt: Option<Date> = None;

        for seg in &self.booking {
            // Allows to advance to the first valid segment
            if seg.end <= at_time {
                continue;
            }

            // Segment is not valid, we need to reset the building process with the next segment
            if bundle.priority <= seg.val {
                tx_end_opt = None;
                continue;
            }
            // Start building or pursue ?
            match tx_end_opt {
                // Try to pursue the build process
                Some(tx_end) => {
                    // the seg is valid, check if this is the last one to consider
                    if tx_end < seg.end {
                        let delay = super::get_delay(tx_end, &self.delay_intervals);
                        return Some(ContactManagerTxData {
                            tx_start,
                            tx_end,
                            delay,
                            expiration: seg.end,
                            arrival: tx_end + delay,
                        });
                    }
                    // if we reach this point, the seg is valid, but transmission didn't reach terminaison, check next
                }
                // (re)-start the build process
                None => {
                    tx_start = Date::max(seg.start, at_time);
                    // In most cases, there should be a single rate seg
                    if let Some(tx_end) = super::get_tx_end(
                        &self.rate_intervals,
                        tx_start,
                        bundle.size,
                        contact_data.end,
                    ) {
                        tx_end_opt = Some(tx_end);
                    };
                }
            }
        }
        None
    }

    fn schedule_tx(
        &mut self,
        contact_data: &ContactInfo,
        at_time: Date,
        bundle: &Bundle,
    ) -> Option<ContactManagerTxData> {
        let out = self.dry_run_tx(contact_data, at_time, bundle)?;
        let tx_start = out.tx_start;
        let tx_end = out.tx_end;

        let mut i = 0;
        while i < self.booking.len() {
            let seg = &self.booking[i];

            // Segment completely before tx window
            if seg.end <= tx_start {
                i += 1;
                continue;
            }

            // Segment completely after tx window
            if seg.start >= tx_end {
                break;
            }

            let old_prio = seg.val;

            // Cut before
            if seg.start < tx_start {
                let left = Segment {
                    start: seg.start,
                    end: tx_start,
                    val: old_prio,
                };
                self.booking.insert(i, left);
                self.booking[i + 1].start = tx_start;
                i += 1;
            }

            // Cut after
            if self.booking[i].end > tx_end {
                let right = Segment {
                    start: tx_end,
                    end: self.booking[i].end,
                    val: old_prio,
                };
                self.booking.insert(i + 1, right);
                self.booking[i].end = tx_end;
            }

            // Assign TX priority
            self.booking[i].val = bundle.priority;
            i += 1;
        }

        Some(out)
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
            &mut self.booking,
            -1,
            #[cfg(feature = "first_depleted")]
            &mut self.original_volume,
            contact_data,
        )
    }
}

/// Implements the `Parser` trait for `SegmentationManager`, allowing the manager to be parsed from a lexer.
impl Parser<PSegmentationManager> for PSegmentationManager {
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
    fn parse(lexer: &mut dyn Lexer) -> ParsingState<PSegmentationManager> {
        super::parse::<PSegmentationManager>(lexer)
    }
}
