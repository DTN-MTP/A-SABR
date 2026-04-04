#[cfg(feature = "first_depleted")]
use crate::types::Volume;
use crate::{
    bundle::Bundle,
    contact::ContactInfo,
    contact_manager::{
        ContactManager, ContactManagerTxData,
        segmentation::{BaseSegmentationManager, Segment},
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

            let (d_start, d_end) = super::get_delays(tx_start, tx_end, &self.delay_intervals);
            return Some(ContactManagerTxData {
                tx_start,
                tx_end,
                expiration: free_seg.end,
                rx_start: tx_start + d_start,
                rx_end: tx_end + d_end,
            });
        }
        None
    }

    /// Schedule the transmission of a bundle by splitting the available free intervals accordingly.
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
        let (d_start, d_end) = super::get_delays(tx_start, tx_end, &self.delay_intervals);

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
            expiration,
            rx_start: tx_start + d_start,
            rx_end: tx_end + d_end,
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
    ///
    /// # Returns
    ///
    /// Returns a `ParsingState` indicating whether parsing was successful (`Finished`) or encountered an error (`Error`).
    fn parse(lexer: &mut dyn Lexer) -> ParsingState<SegmentationManager> {
        super::parse::<SegmentationManager>(lexer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bundle::Bundle, contact::ContactInfo, contact_manager::ContactManager};

    // Just a helper enum to easily define input segments (rate + delay)
    #[derive(Debug, PartialEq, Clone)]
    enum InputSeg {
        Delay(Date, Date, Duration),
        Rate(Date, Date, DataRate),
    }

    // Used to check the remaining free intervals after scheduling
    #[derive(Debug, PartialEq, Clone)]
    enum OutputSeg {
        Free(Date, Date),
    }

    #[track_caller]
    fn start_test(
        contact_start: Date,
        contact_end: Date,
        input: Vec<InputSeg>,
        output: Vec<OutputSeg>,
        requests: Vec<(Bundle, f64, bool)>,
    ) {
        // Create the contact
        let contact_info = ContactInfo::new(0, 1, contact_start, contact_end);

        // Separate delay and rate segments
        let mut delay_segments: Vec<Segment<Duration>> = Vec::new();
        let mut rate_segments: Vec<Segment<DataRate>> = Vec::new();

        // Convert input into actual Segment structs
        for seg in input {
            match seg {
                InputSeg::Delay(start, end, val) => {
                    delay_segments.push(Segment { start, end, val })
                }
                InputSeg::Rate(start, end, val) => rate_segments.push(Segment { start, end, val }),
            }
        }

        // Create the segmentation manager
        let mut manager = SegmentationManager::new(rate_segments, delay_segments);

        // Init should work if segments are valid
        assert!(manager.try_init(&contact_info));

        // Test each request (bundle)
        for (i, (bundle, at_time, expect_success)) in requests.iter().enumerate() {
            let dry_run_res = manager.dry_run_tx(&contact_info, *at_time, bundle);

            // Check if dry_run behaves as expected
            assert_eq!(
                dry_run_res.is_some(),
                *expect_success,
                "TEST N°{} FAILED: expected {} but got {}.",
                i,
                expect_success,
                dry_run_res.is_some()
            );

            // If it should succeed -> test schedule_tx too
            if *expect_success {
                let schedule_tx_res = manager.schedule_tx(&contact_info, *at_time, bundle);

                assert!(
                    schedule_tx_res.is_some(),
                    "TEST N°{} FAILED: schedule_tx failed unexpectedly.",
                    i
                );

                assert_eq!(
                    dry_run_res.is_some(),
                    schedule_tx_res.is_some(),
                    "TEST N°{} FAILED: dry_run_tx and schedule_tx do not match.",
                    i
                );

                if let (Some(dry), Some(sched)) = (dry_run_res, schedule_tx_res) {
                    assert_eq!(
                        dry.tx_start, sched.tx_start,
                        "TEST N°{} FAILED: tx_start mismatch.",
                        i
                    );
                    assert_eq!(
                        dry.tx_end, sched.tx_end,
                        "TEST N°{} FAILED: tx_end mismatch.",
                        i
                    );
                    assert_eq!(
                        dry.expiration, sched.expiration,
                        "TEST N°{} FAILED: expiration mismatch.",
                        i
                    );
                    assert_eq!(
                        dry.rx_start, sched.rx_start,
                        "TEST N°{} FAILED: rx_start mismatch.",
                        i
                    );
                    assert_eq!(
                        dry.rx_end, sched.rx_end,
                        "TEST N°{} FAILED: rx_end mismatch.",
                        i
                    );
                }
            }
        }

        // Collect the remaining free intervals after all insertions
        let mut actual_output = Vec::new();
        for seg in &manager.free_intervals {
            actual_output.push(OutputSeg::Free(seg.start, seg.end));
        }

        // Compare with expected result
        assert_eq!(
            actual_output, output,
            "TEST FAILED: actual free intervals do not match expected output."
        );
    }

    #[test]
    fn test_single_bundle_insertions() {
        // Simple case: one delay segment + one rate segment
        let input = vec![
            InputSeg::Delay(0.0, 200.0, 4.0),
            InputSeg::Rate(0.0, 200.0, 100.0),
        ];

        // Small bundle -> should fit easily
        let bundle1 = Bundle {
            source: 0,
            destinations: vec![1],
            priority: 1,
            size: 100.0,
            expiration: 1000.0,
        };

        // It uses a small part at the beginning -> remaining is [1,200]
        let output1 = vec![OutputSeg::Free(1.0, 200.0)];
        start_test(
            0.0,
            200.0,
            input.clone(),
            output1,
            vec![(bundle1, 0.0, true)],
        );

        // Bigger bundle -> cuts a chunk in the middle
        let bundle2 = Bundle {
            source: 0,
            destinations: vec![1],
            priority: 1,
            size: 4000.0,
            expiration: 1000.0,
        };

        // Free intervals are now split in two
        let output2 = vec![OutputSeg::Free(0.0, 80.0), OutputSeg::Free(120.0, 200.0)];
        start_test(
            0.0,
            200.0,
            input.clone(),
            output2,
            vec![(bundle2, 80.0, true)],
        );

        // Even bigger bundle -> takes a large portion
        let bundle3 = Bundle {
            source: 0,
            destinations: vec![1],
            priority: 2,
            size: 5000.0,
            expiration: 1000.0,
        };

        let output3 = vec![OutputSeg::Free(0.0, 150.0), OutputSeg::Free(200.0, 200.0)];
        start_test(
            0.0,
            200.0,
            input.clone(),
            output3,
            vec![(bundle3, 150.0, true)],
        );

        // Too large -> should fail and not modify anything
        let bundle_too_large = Bundle {
            source: 0,
            destinations: vec![1],
            priority: 1,
            size: 50_000.0,
            expiration: 1000.0,
        };

        let output4 = vec![OutputSeg::Free(0.0, 200.0)];
        start_test(
            0.0,
            200.0,
            input,
            output4,
            vec![(bundle_too_large, 0.0, false)],
        );
    }

    #[test]
    fn test_multiple_insertions_on_same_contact() {
        let input = vec![
            InputSeg::Delay(0.0, 200.0, 4.0),
            InputSeg::Rate(0.0, 200.0, 100.0),
        ];

        // We insert multiple bundles sequentially
        let bundle1 = Bundle {
            source: 0,
            destinations: vec![1],
            priority: 1,
            size: 1000.0,
            expiration: 1000.0,
        };

        let bundle2 = Bundle {
            source: 0,
            destinations: vec![1],
            priority: 1,
            size: 500.0,
            expiration: 1000.0,
        };

        let bundle3 = Bundle {
            source: 0,
            destinations: vec![1],
            priority: 1,
            size: 1000.0,
            expiration: 1000.0,
        };

        // They should be placed one after another
        let requests = vec![
            (bundle1, 0.0, true),  // [0,10]
            (bundle2, 10.0, true), // [10,15]
            (bundle3, 15.0, true), // [15,25]
        ];

        let output = vec![OutputSeg::Free(25.0, 200.0)];

        start_test(0.0, 200.0, input, output, requests);
    }

    #[test]
    fn test_variable_rate_segments() {
        let input = vec![
            InputSeg::Delay(0.0, 200.0, 4.0),
            InputSeg::Rate(0.0, 50.0, 100.0),
            InputSeg::Rate(50.0, 200.0, 50.0),
        ];

        let bundle = Bundle {
            source: 0,
            destinations: vec![1],
            priority: 1,
            size: 7500.0,
            expiration: 1000.0,
        };

        let requests = vec![(bundle, 0.0, true)];

        // First part is fast (0–50), then slower -> finishes at 100
        let output = vec![OutputSeg::Free(100.0, 200.0)];

        start_test(0.0, 200.0, input, output, requests);
    }

    #[test]
    fn test_start_time_handling() {
        let input = vec![
            InputSeg::Delay(5.0, 15.0, 1.0),
            InputSeg::Rate(5.0, 15.0, 2.0),
        ];

        let bundle = Bundle {
            source: 0,
            destinations: vec![1],
            priority: 1,
            size: 4.0,
            expiration: 1000.0,
        };

        let requests = vec![
            (bundle, 0.0, true), // should start at contact start (5.0)
        ];

        // It uses [5,7], so remaining is [7,15]
        let output = vec![OutputSeg::Free(7.0, 15.0)];

        start_test(5.0, 15.0, input, output, requests);
    }
}
