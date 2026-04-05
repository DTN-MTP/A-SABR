#[cfg(feature = "first_depleted")]
use crate::types::Volume;
use crate::{
    bundle::Bundle,
    contact::ContactInfo,
    contact_manager::{
        ContactManager, ContactManagerTxData,
        segmentation::{BaseSegmentationManager, Segment},
    },
    parsing::{Lexer, Parser, ParsingState},
    types::{DataRate, Date, Duration, Priority},
};

/// Priority-aware segmentation manager. Tracks bandwidth availability per priority level
/// using booking intervals.
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct PSegmentationManager {
    /// A list of segments tracking the priority level booked for each time interval.
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
    /// Simulates the transmission of a bundle based on the contact data and bundle priority.
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
                    if tx_end <= seg.end {
                        let (d_start, d_end) =
                            super::get_delays(tx_start, tx_end, &self.delay_intervals);
                        return Some(ContactManagerTxData {
                            tx_start,
                            tx_end,
                            expiration: seg.end,
                            rx_start: tx_start + d_start,
                            rx_end: tx_end + d_end,
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
                        if tx_end <= seg.end {
                            let (d_start, d_end) =
                                super::get_delays(tx_start, tx_end, &self.delay_intervals);
                            return Some(ContactManagerTxData {
                                tx_start,
                                tx_end,
                                expiration: seg.end,
                                rx_start: tx_start + d_start,
                                rx_end: tx_end + d_end,
                            });
                        }
                        tx_end_opt = Some(tx_end);
                    };
                }
            }
        }
        None
    }

    /// Schedule the transmission of a bundle by updating the booking intervals with the bundle's priority.
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

/// Implements the `Parser` trait for `PSegmentationManager`, allowing the manager to be parsed from a lexer.
impl Parser<PSegmentationManager> for PSegmentationManager {
    /// Parses a `PSegmentationManager` from the lexer, extracting the rate and delay intervals.
    ///
    /// # Arguments
    ///
    /// * `lexer` - The lexer used for parsing tokens.
    ///
    /// # Returns
    ///
    /// Returns a `ParsingState` indicating whether parsing was successful (`Finished`) or encountered an error (`Error`).
    fn parse(lexer: &mut dyn Lexer) -> ParsingState<PSegmentationManager> {
        super::parse::<PSegmentationManager>(lexer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::Bundle;
    use crate::contact::ContactInfo;
    use crate::contact_manager::ContactManager;
    use crate::contact_manager::segmentation::Segment;
    use crate::types::{Date, Duration};

    #[derive(Debug, PartialEq, Clone)]
    enum InputSeg {
        Delay(Date, Date, Duration),
        Rate(Date, Date, DataRate),
        Booking(Date, Date, Priority),
    }

    #[derive(Debug, PartialEq, Clone)]
    enum OutputSeg {
        Booking(Date, Date, Priority),
    }

    #[track_caller]
    fn start_test(
        input: Vec<InputSeg>,
        output: Vec<OutputSeg>,
        requests: Vec<(Bundle, f64, bool)>,
    ) {
        let contact_info = ContactInfo::new(0, 1, 0.0, 200.0);
        let mut delay_segments: Vec<Segment<Date>> = Vec::new();
        let mut rate_segments: Vec<Segment<DataRate>> = Vec::new();
        let mut actual_output = Vec::new();
        let mut initial_bookings: Vec<Segment<Priority>> = Vec::new();

        for seg in input {
            match seg {
                InputSeg::Delay(start, end, val) => {
                    delay_segments.push(Segment { start, end, val })
                }
                InputSeg::Rate(start, end, val) => rate_segments.push(Segment { start, end, val }),
                InputSeg::Booking(start, end, priority) => initial_bookings.push(Segment {
                    start,
                    end,
                    val: priority,
                }),
            }
        }
        let mut manager = PSegmentationManager::new(rate_segments, delay_segments);
        manager.try_init(&contact_info);
        if !initial_bookings.is_empty() {
            manager.booking = initial_bookings;
        }

        for (i, (bundle, at_time, expect_success)) in requests.iter().enumerate() {
            let dry_run_res = manager.dry_run_tx(&contact_info, *at_time, bundle);
            let schedule_tx_res = manager.schedule_tx(&contact_info, *at_time, bundle);

            assert_eq!(
                dry_run_res, schedule_tx_res,
                "TEST N°{} FAILED: dry_run and schedule_tx doesn't match.\n",
                i
            );
            assert_eq!(
                schedule_tx_res.is_some(),
                *expect_success,
                "TEST N°{} FAILED: expected: {} actual: {}",
                i,
                expect_success,
                schedule_tx_res.is_some()
            );
        }

        //Building actual output

        for seg in &manager.booking {
            actual_output.push(OutputSeg::Booking(seg.start, seg.end, seg.val));
        }
        assert_eq!(
            actual_output, output,
            "TEST FAILED: Actual output is not the one expected."
        );
    }

    #[test]
    fn test_single_bundle_insertion() {
        let bundle1 = Bundle {
            source: 0,
            destinations: vec![1],
            priority: 1,
            size: 100.0,
            expiration: 1000.0,
        };
        let input = vec![
            InputSeg::Delay(0.0, 200.0, 4.0),
            InputSeg::Rate(0.0, 200.0, 100.0),
        ];

        let output1 = vec![
            OutputSeg::Booking(0.0, 1.0, 1),
            OutputSeg::Booking(1.0, 200.0, -1),
        ];
        start_test(input.clone(), output1, vec![(bundle1, 0.0, true)]);
        // Time (T) : 0 .................................................... 200
        // Network  : [------------------------------------------------------]
        //            (Rate and Delay continuously available)
        //
        // Request  : [X] (Priority 1 bundle arrives at T=0, needs 1s)
        //             |
        //             V
        // Booking  : [X][---------------------------------------------------]
        //   Priority: 1                          -1
        //         (0 to 1)                   (1 to 200)

        let bundle2 = Bundle {
            source: 0,
            destinations: vec![1],
            priority: 1,
            size: 4000.0,
            expiration: 1000.0,
        };
        let output2 = vec![
            OutputSeg::Booking(0.0, 80.0, -1),
            OutputSeg::Booking(80.0, 120.0, 1),
            OutputSeg::Booking(120.0, 200.0, -1),
        ];
        start_test(input.clone(), output2, vec![(bundle2, 80.0, true)]);
        // =====================================================================
        // SCENARIO: Future Insertion (at_time = 80.0)
        // Request: Bundle 2 (Size 4000, Prio 1, at T=80.0) -> Needs 40.0s
        //
        // Time (T) : 0 ........................ 80 ...... 120 ............. 200
        // Network  : [------------------------------------------------------]
        //            (Rate and Delay continuously available)
        //
        // Request  :                            [XXXXXXXXX] (Priority 1 bundle)
        //                                            |
        //                                            V
        // Booking  : [-------------------------][XXXXXXXXX][-----------------]
        // Priority :             -1                  1              -1
        //                     (0 to 80)         (80 to 120)    (120 to 200)
        // =====================================================================

        let bundle3 = Bundle {
            source: 0,
            destinations: vec![1],
            priority: 2,
            size: 5000.0,
            expiration: 1000.0,
        };
        let output3 = vec![
            OutputSeg::Booking(0.0, 150.0, -1),
            OutputSeg::Booking(150.0, 200.0, 2),
        ];
        start_test(input.clone(), output3, vec![(bundle3, 150.0, true)]);
        // =====================================================================
        // SCENARIO: Exact Fit at the End
        // Request: Bundle 3 (Size 5000, Prio 2, at T=150.0) -> Needs 50.0s
        //
        // Time (T) : 0 ................................. 150 .......... 200
        // Network  : [------------------------------------------------------]
        //            (Rate and Delay continuously available)
        //
        // Request  :                                     [XXXXXXXXXX] (Priority 2)
        //                                                     |
        //                                                     V
        // Booking  : [-----------------------------------][XXXXXXXXXX]
        // Priority :                  -1                        2
        //                         (0 to 150)              (150 to 200)
        // =====================================================================

        let bundle_too_large = Bundle {
            source: 0,
            destinations: vec![1],
            priority: 1,
            size: 50_000.0,
            expiration: 1000.0,
        };

        let requests = vec![(bundle_too_large, 0.0, false)];

        let output = vec![OutputSeg::Booking(0.0, 200.0, -1)];

        start_test(input, output, requests);
        // =====================================================================
        // SCENARIO: Capacity Exceeded (None failure)
        // Request: Bundle Too Large (Size 50,000 | Max Network Capacity 20,000)
        //
        // Time (T) : 0 ................................................. 200
        // Network  : [====================================================]
        // Capacity : <---------- Can only hold 20,000 units -------------->
        //
        // Request  : [XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX...]
        //            <------------------ Needs 50,000 units ------------------------------>
        //
        // Result   : The loop reaches T=200, still has 30,000 units to send.
        //            It hits the final 'None' because it's out of time!
        //
        // Booking  : [----------------------- -1 -------------------------] (Unchanged)
        // =====================================================================
    }

    #[test]
    fn test_bundles_priorities() {
        let input = vec![
            InputSeg::Delay(0.0, 200.0, 4.0),
            InputSeg::Rate(0.0, 200.0, 100.0),
        ];

        let bundle_prio_1 = Bundle {
            source: 0,
            destinations: vec![1],
            priority: 1,
            size: 10000.0,
            expiration: 1000.0,
        };
        let bundle_prio_0 = Bundle {
            source: 0,
            destinations: vec![1],
            priority: 0,
            size: 1000.0,
            expiration: 1000.0,
        };
        let bundle_prio_2 = Bundle {
            source: 0,
            destinations: vec![1],
            priority: 2,
            size: 100.0,
            expiration: 1000.0,
        };

        let requests = vec![
            (bundle_prio_1, 0.0, true),
            (bundle_prio_2, 50.0, true),
            (bundle_prio_0, 50.0, true),
        ];

        let output = vec![
            OutputSeg::Booking(0.0, 50.0, 1),
            OutputSeg::Booking(50.0, 51.0, 2),
            OutputSeg::Booking(51.0, 100.0, 1),
            OutputSeg::Booking(100.0, 110.0, 0),
            OutputSeg::Booking(110.0, 200.0, -1),
        ];

        start_test(input, output, requests);
        // =====================================================================
        // SCENARIO: Triple Priority Battle (Preemption & Postponing)
        //
        // 1. T=0.0  : Bundle Prio 1 arrives -> Reserves [0 to 100]
        // 2. T=50.0 : Bundle Prio 2 arrives -> Higher than Prio 1? YES.
        //             It slices Prio 1 in half and takes [50 to 51].
        // 3. T=50.0 : Bundle Prio 0 arrives -> Higher than Prio 2 or 1? NO.
        //             It searches for free space and finds it after Prio 1 ends.
        //
        // Time (T) : 0          50   51          100       110             200
        //            |----------|----|-----------|---------|---------------|
        //
        // Network  : [=====================================================]
        //
        // Step 1   : [XXXXXXXXXX Prio 1 XXXXXXXXX]
        // Step 2   : [--- 1 ----][ 2 ][---- 1 ---]  <-- (Prio 2 preempted 1)
        // Step 3   : [--- 1 ----][ 2 ][---- 1 ---][ 0 ] <-- (Prio 0 postponed)
        //
        // Final Booking State:
        // Segment 1: [0.0  - 50.0 ] -> Prio 1
        // Segment 2: [50.0 - 51.0 ] -> Prio 2
        // Segment 3: [51.0 - 100.0] -> Prio 1
        // Segment 4: [100.0- 110.0] -> Prio 0
        // Segment 5: [110.0- 200.0] -> Free (-1)
        // =====================================================================
    }

    #[test]
    fn test_overlapping_multiple_segments() {
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

        let output = vec![
            OutputSeg::Booking(0.0, 100.0, 1),
            OutputSeg::Booking(100.0, 200.0, -1),
        ];

        start_test(input, output, requests);
        // =====================================================================
        // SCENARIO: Single Bundle across Variable Data Rates
        //
        // 1. T=0.0 to 50.0  : Rate is 100 bps.
        //                     In 50s, we send 5000 units.
        //                     Remaining size: 7500 - 5000 = 2500 units.
        //
        // 2. T=50.0 to 100.0 : Rate drops to 50 bps.
        //                     To send the remaining 2500 units, we need:
        //                     2500 / 50 = 50s.
        //                     End time: 50.0 + 50.0 = 100.0.
        //
        // Time (T) : 0          50                 100                200
        //            |----------|------------------|------------------|
        //
        // Rate     : [ 100 bps  ][     50 bps      ][     50 bps     ]
        //
        // Bundle   : [XXXXXXXXXX][XXXXXXXXXXXXXXXXX]
        //              (5000)         (2500)
        //
        // Final Booking State:
        // Segment 1: [0.0  - 100.0] -> Prio 1 (The full transmission)
        // Segment 2: [100.0 - 200.0] -> Free (-1)
        // =====================================================================
    }

    #[test]
    fn test_preemption_across_multiple_segments() {
        let input = vec![
            InputSeg::Delay(0.0, 200.0, 4.0),
            InputSeg::Rate(0.0, 200.0, 100.0),
        ];

        let bundle_preempted = Bundle {
            source: 0,
            destinations: vec![1],
            priority: 1,
            size: 1000.0,
            expiration: 1000.0,
        };

        let bundle_preempting_large = Bundle {
            source: 0,
            destinations: vec![1],
            priority: 2,
            size: 3000.0,
            expiration: 1000.0,
        };

        let requests = vec![
            (bundle_preempted, 10.0, true),
            (bundle_preempting_large, 0.0, true),
        ];

        let output = vec![
            OutputSeg::Booking(0.0, 10.0, 2),
            OutputSeg::Booking(10.0, 20.0, 2),
            OutputSeg::Booking(20.0, 30.0, 2),
            OutputSeg::Booking(30.0, 200.0, -1),
        ];

        start_test(input, output, requests);
        // =====================================================================
        // SCENARIO: Multi-Segment Preemption
        //
        // 1. Initial State: A small bundle (Prio 1) is placed at T=10 to T=20.
        //    Booking: [ 0 -- 10: Free ] [ 10 -- 20: Prio 1 ] [ 20 -- 200: Free ]
        //
        // 2. Event: A large VIP bundle (Prio 2) arrives at T=0.
        //    Size: 3000 | Rate: 100 => Needs 30.0s duration.
        //
        // 3. Execution: Prio 2 "bulldozes" through three different segments:
        //    - Segment A [0-10] (Free)    -> Overwritten by Prio 2
        //    - Segment B [10-20] (Prio 1) -> Preempted by Prio 2
        //    - Segment C [20-200] (Free)   -> First 10s taken by Prio 2
        //
        // Time (T) : 0          10          20          30                 200
        //            |----------|-----------|-----------|------------------|
        //
        // Before   : [  Free -1 ][  Prio 1  ][         Free -1             ]
        //
        // After    : [  Prio 2  ][  Prio 2  ][  Prio 2  ][     Free -1      ]
        //             (from Seg A)(from Seg B)(from Seg C)
        //
        // Final Booking State (reflecting the "scars" of previous segments):
        // Segment 1: [0.0  - 10.0 ] -> Prio 2
        // Segment 2: [10.0 - 20.0 ] -> Prio 2
        // Segment 3: [20.0 - 30.0 ] -> Prio 2
        // Segment 4: [30.0 - 200.0] -> Free (-1)
        // =====================================================================
    }

    #[test]
    fn test_existing_booking() {
        let input = vec![
            InputSeg::Delay(0.0, 200.0, 4.0),
            InputSeg::Rate(0.0, 200.0, 100.0),
            InputSeg::Booking(0.0, 50.0, -1),
            InputSeg::Booking(50.0, 100.0, 1),
            InputSeg::Booking(100.0, 200.0, -1),
        ];

        let bundle = Bundle {
            source: 0,
            destinations: vec![1],
            priority: 2,
            size: 1000.0,
            expiration: 1000.0,
        };

        let requests = vec![(bundle, 60.0, true)];

        let output = vec![
            OutputSeg::Booking(0.0, 50.0, -1),
            OutputSeg::Booking(50.0, 60.0, 1),
            OutputSeg::Booking(60.0, 70.0, 2),
            OutputSeg::Booking(70.0, 100.0, 1),
            OutputSeg::Booking(100.0, 200.0, -1),
        ];

        start_test(input, output, requests);
        // =====================================================================
        //SCENARIO: Booking Not Empty at Initialization (0.0 to 200.0)

        //1. Initial State:
        //   Booking: [ 0.0 -- 50.0: Free ] [ 50.0 -- 100.0: Prio 1 ] [ 100.0 -- 200.0: Free ]

        //2. Event:
        //   A new bundle (Priority 2) arrives at T=60.0.
        //   Needs 10.0s (Size 1000 / Rate 100).

        //3. Execution:
        //   Prio 2 is higher than Prio 1. It preempts the existing booking
        //   exactly between T=60.0 and T=70.0.

        //Time (T) : 0          50          60          70          100         200
        //           |----------|-----------|-----------|-----------|-----------|

        //Before   : [ Free -1 ][        Priority 1        ][      Free -1      ]

        //Request  :                         [ Prio 2 ]
        //                                      |
        //                                      V

        //After    : [ Free -1 ][  Prio 1  ][  Prio 2  ][  Prio 1  ][  Free -1  ]
        //            (0.0-50)   (50-60)     (60-70)     (70-100)   (100-200)

        //Final Booking State:
        //- Segment 1: [0.0  - 50.0 ] -> Free (-1)
        //- Segment 2: [50.0 - 60.0 ] -> Priority 1
        //- Segment 3: [60.0 - 70.0 ] -> Priority 2 (PREEMPTION)
        //- Segment 4: [70.0 - 100.0] -> Priority 1
        //- Segment 5: [100.0- 200.0] -> Free (-1)
        //=====================================================================

        let input = vec![
            InputSeg::Delay(0.0, 200.0, 4.0),
            InputSeg::Rate(0.0, 200.0, 100.0),
            InputSeg::Booking(0.0, 20.0, 1),
            InputSeg::Booking(20.0, 80.0, -1),
            InputSeg::Booking(80.0, 100.0, 1),
            InputSeg::Booking(100.0, 200.0, -1),
        ];

        let bundle_low_prio = Bundle {
            source: 0,
            destinations: vec![1],
            priority: 0,
            size: 3000.0,
            expiration: 1000.0,
        };

        let requests = vec![(bundle_low_prio, 10.0, true)];

        let output = vec![
            OutputSeg::Booking(0.0, 20.0, 1),
            OutputSeg::Booking(20.0, 50.0, 0),
            OutputSeg::Booking(50.0, 80.0, -1),
            OutputSeg::Booking(80.0, 100.0, 1),
            OutputSeg::Booking(100.0, 200.0, -1),
        ];

        start_test(input, output, requests);
        // =====================================================================
        // SCENARIO: Gap Filling (Low Priority)
        //
        // 1. Initial State: Segments [0-20] and [80-100] are Prio 1.
        // 2. Event: Prio 0 bundle arrives at T=10.
        // 3. Execution: T=10 is BUSY with Prio 1 (higher).
        //    The manager skips to the first available slot at T=20.
        //
        // Time (T) : 0     10     20               50               80     100     200
        //            |-----|------|----------------|----------------|------|-------|
        //
        // Before   : [ Prio 1 ]    [     FREE     ]                [ Prio 1 ]
        //
        // Request  :       [X] (Arrives at 10, Prio 0)
        //                   |
        //                   V (Wait for free space...)
        //
        // After    : [ Prio 1 ]    [ Prio 0 (30s) ] [     FREE     ][ Prio 1 ]
        //             (0-20)        (20-50)          (50-80)         (80-100)
        // =====================================================================

        let input = vec![
            InputSeg::Delay(0.0, 200.0, 4.0),
            InputSeg::Rate(0.0, 200.0, 100.0),
            InputSeg::Booking(0.0, 100.0, 0),
            InputSeg::Booking(100.0, 120.0, 1),
            InputSeg::Booking(120.0, 200.0, -1),
        ];

        let bundle_prio2 = Bundle {
            source: 0,
            destinations: vec![1],
            priority: 2,
            size: 8000.0,
            expiration: 1000.0,
        };

        let requests = vec![(bundle_prio2, 10.0, true)];

        let output = vec![
            OutputSeg::Booking(0.0, 10.0, 0),
            OutputSeg::Booking(10.0, 90.0, 2),
            OutputSeg::Booking(90.0, 100.0, 0),
            OutputSeg::Booking(100.0, 120.0, 1),
            OutputSeg::Booking(120.0, 200.0, -1),
        ];

        start_test(input, output, requests);
        //=====================================================================
        //SCENARIO: Partial Preemption of a Long Segment

        //1. Initial State:
        //   A long Priority 0 segment exists from T=0.0 to T=100.0.
        //   Booking: [ 0.0 -- 100.0: Prio 0 ] [ 100.0 -- 120.0: Prio 1 ] [ 120.0 -- 200.0: -1 ]

        //2. Event:
        //   Bundle (Priority 2) arrives at T=10.0.
        //   Size 8000 / Rate 100 => Duration 80.0s.
        //   Transmission window: [10.0 to 90.0].

        //3. Execution:
        //   Prio 2 > Prio 0. It carves its space inside the first segment.
        //   The end of the first segment (90.0 to 100.0) remains Prio 0.

        //Time (T) : 0     10                           90    100    120         200
        //           |-----|----------------------------|-----|------|-----------|

        //Before   : [        Priority 0 (0-100)        ][ P1 ][     Free -1     ]

        //Request  :       [XXXXXXXX Prio 2 (80s) XXXXXX]
        //                  (10 to 90)

        //After    : [ P0 ][      Priority 2 (10-90)    ][ P0 ][ P1 ][  Free -1  ]
        //            (0-10)      (The "Bulldozer")      (90-100)

        //Final Booking State:
        //- Segment 1: [0.0  - 10.0 ] -> Priority 0 (Prefix)
        //- Segment 2: [10.0 - 90.0 ] -> Priority 2 (New Bundle)
        //- Segment 3: [90.0 - 100.0] -> Priority 0 (Suffix)
        //- Segment 4: [100.0- 120.0] -> Priority 1 (Untouched)
        //- Segment 5: [120.0- 200.0] -> Free (-1)
        //=====================================================================
    }
}
