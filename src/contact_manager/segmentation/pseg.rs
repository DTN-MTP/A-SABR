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
                    if tx_end < seg.end{
                        let delay = super::get_delay(tx_end,&self.delay_intervals);
                        return Some(ContactManagerTxData{
                            tx_start,
                            tx_end,
                            delay,
                            expiration: seg.end,
                            arrival: tx_end + delay,

                        })
                    }
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


#[cfg(test)]
mod tests{
    use super::*;
    use crate::types::{Date, Duration};
    use crate::contact_manager::segmentation::Segment;
    use crate::contact_manager::ContactManager;
    use crate::contact::ContactInfo;
    use crate::bundle::Bundle;

    fn mock_contact_info() -> ContactInfo{
        ContactInfo::new(
            0,
            1,
            0.0,
            100.0,
        )
    }

    fn setup_manager(rate_segments: Vec<Segment<Date>>, delay_segments: Vec<Segment<Date>>) -> PSegmentationManager{
        
        let mut mgr = PSegmentationManager::new(rate_segments,delay_segments);

        mgr.try_init(&mock_contact_info());
        mgr
    }
      struct MockLexer{
        tokens: Vec<String>,
        position: usize,
    }

    impl MockLexer {
        fn new(data: Vec<&str>) -> Self {
            Self {
                tokens: data.iter().map(|s| s.to_string()).collect(),
                position: 0,
            }
        }
    }


    impl Lexer for MockLexer{
        fn lookup(&mut self) -> ParsingState<String>{
            if self.position < self.tokens.len(){
                ParsingState::Finished(self.tokens[self.position].clone())
            }
            else {
                ParsingState::EOF
            }
        }
        fn consume_next_token(&mut self) -> ParsingState<String> {
            let state = self.lookup();
            if let ParsingState::Finished(_) = state {
                self.position += 1;
            }
            state
        }
        fn get_current_position(&self) -> String {
            format!("Token at index {}",self.position)
        }
    }

    #[test]
    fn test_higher_priority_replace_lower_priority(){
        let rate_segments: Vec<Segment<Date>> = vec![
            Segment{
                start: 0.0,
                end: 100.0,
                val: 1000.0,
            }
        ];
        let delay_segments: Vec<Segment<Duration>> = vec![
            Segment{
                start: 0.0,
                end: 100.0,
                val: 1.0,
            }
        ];

        let mut mgr = setup_manager(rate_segments,delay_segments);
        let contact_info = mock_contact_info();

        let normal_bundle = Bundle{
            source: 0,
            destinations: vec![1],
            priority: 0,
            size: 10000.0,
            expiration: 2000.0,
        };
        let normal_res = mgr.schedule_tx(&contact_info, 0.0, &normal_bundle);

        //Normal priority (0) bundle should take the segment
        assert!(normal_res.is_some(),"schedule_tx method for normal priority failed");

        let urgent_bundle = Bundle{
            source: 0,
            destinations:vec![1],
            priority: 2,
            size: 8000.0,
            expiration: 1800.0,
        };
        let urgent_res = mgr.schedule_tx(&contact_info, 0.0, &urgent_bundle);

        //Urgent priority (2) bundle should take the segment already occupied by normal priority bundle
        assert!(urgent_res.is_some(),"schedule_tx method for urgent priority failed");
    }

    #[test]
    fn test_lower_priority_dont_replace_higher_priority(){
        let rate_segments: Vec<Segment<Date>> = vec![
            Segment{
                start: 0.0,
                end: 100.0,
                val: 1000.0,
            }
        ];
        let delay_segments: Vec<Segment<Duration>> = vec![
            Segment{
                start: 0.0,
                end: 100.0,
                val: 1.0,
            }
        ];

        let mut mgr = setup_manager(rate_segments,delay_segments);
        let contact_info = mock_contact_info();
        let urgent_bundle = Bundle {
            source: 0,
            destinations: vec![1],
            priority: 2,
            size: 80000.0,
            expiration: 1600.0,
        };
        //Urgent bundle takes the only segment entirely
        let res_urgent = mgr.schedule_tx(&contact_info, 0.0, &urgent_bundle);
        assert!(res_urgent.is_some(),"Urgent bundle should fit");

        let normal_bundle = Bundle{
            source: 0,
            destinations: vec![1],
            priority: 0,
            size: 30000.0,
            expiration: 1700.0,
        };
        let add_normal_bundle_res = mgr.schedule_tx(&contact_info, 0.0, &normal_bundle);
        assert!(add_normal_bundle_res.is_none(),"Normal priority bundle shouldn't take urgent bundle segment.");

    }
    #[test]
    fn test_bundles_takes_the_same_segment(){
        let rate_segments: Vec<Segment<Date>> = vec![
            Segment{
                start: 0.0,
                end: 100.0,
                val: 1000.0,
            }
        ];
        let delay_segments: Vec<Segment<Duration>> = vec![
            Segment{
                start: 0.0,
                end: 100.0,
                val: 1.0,
            }
        ];

        let mut mgr = setup_manager(rate_segments,delay_segments);
        let contact_info = mock_contact_info();

        let bundle1 = Bundle{
            source: 0,
            destinations: vec![1],
            priority: 1,
            size: 50000.0,
            expiration: 2000.0,
        };
        let bundle2 = Bundle{
            source: 0,
            destinations: vec![1],
            priority: 1,
            size: 49000.0,
            expiration: 2000.0,
        };

        //Add first bundle
        let first_result = mgr.schedule_tx(&contact_info, 0.0, &bundle1);
        assert!(first_result.is_some(), "First bundle should have been added.");

        //Add second bundle
        let second_result = mgr.schedule_tx(&contact_info, 0.0, &bundle2);
        assert!(second_result.is_some(), "Second bundle should have been added.");

    }

    #[test]
    fn test_match_dry_run_schedule_tx(){
        let rate_segments: Vec<Segment<Date>> = vec![
            Segment{
                start: 0.0,
                end: 70.0,
                val: 1000.0,
            },
            Segment{
                start: 70.0,
                end: 100.0,
                val: 800.0,
            }
        ];
        let delay_segments: Vec<Segment<Duration>> = vec![
            Segment{
                start: 0.0,
                end: 100.0,
                val: 1.0,
            }
        ];

        let mut mgr = setup_manager(rate_segments,delay_segments);
        let contact_info = mock_contact_info();

        let bundle = Bundle{
            source: 0,
            destinations: vec![1],
            priority: 1,
            size: 50000.0,
            expiration: 2000.0,
        };

        let dry_run_result = mgr.dry_run_tx(&contact_info,0.0,&bundle).unwrap();
        let schedule_result =  mgr.schedule_tx(&contact_info,0.0,&bundle).unwrap();

        //Checking if fields match each other
        assert!(dry_run_result.tx_start == schedule_result.tx_start, "tx_start fields doesn't match");
        assert!(dry_run_result.tx_end == schedule_result.tx_end, "tx_end fields doesn't match");
        assert!(dry_run_result.delay == schedule_result.delay, "delay fields doesn't match");
        assert!(dry_run_result.expiration == schedule_result.expiration, "expiration fields doesn't match");
        assert!(dry_run_result.arrival == schedule_result.arrival, "arrival fields doesn't match");
    }

    #[test]
    fn test_bundle_takes_two_segments(){
        let rate_segments: Vec<Segment<Date>> = vec![
            Segment{
                start: 0.0,
                end: 50.0,
                val: 1000.0,
            },
            Segment{
                start: 50.0,
                end: 100.0,
                val: 50.0,
            }
        ];

        let delay_segments: Vec<Segment<Duration>> = vec![
            Segment{
                start: 0.0,
                end: 100.0,
                val: 1.0,
            }
        ];

        let contact_info = mock_contact_info();
        let mut mgr = setup_manager(rate_segments, delay_segments);
        let bundle = Bundle{
            source: 0,
            destinations: vec![1],
            priority: 1,
            size: 50_250.0,
            expiration: 1000.0,
        };

        let result = mgr.schedule_tx(&contact_info, 0.0, &bundle);
        assert!(result.is_some(),"schedule_tx method failed.");

        let data = result.unwrap();
        //50s for first 50_000 bundle's bytes, first rate segment is fully used.
        //250 bytes left at 50 bytes/s = 5s
        //Expected result = 50 + 5 = 55s 
        assert!(data.tx_end == 55.0, "tx_end field is incorrect.");
    }

    #[test]
    fn test_high_priority_bundle_overwrites_multiple_low_priority_bundles(){
        let rate_segments: Vec<Segment<Date>> = vec![
            Segment{
                start: 0.0,
                end: 50.0,
                val: 100.0,
            },
            Segment{
                start: 50.0,
                end: 100.0,
                val: 100.0,
            }
        ];

        let delay_segments: Vec<Segment<Duration>> = vec![
            Segment{
                start: 0.0,
                end: 100.0,
                val: 1.0,
            }
        ];
        let mut mgr = setup_manager(rate_segments, delay_segments);
        let contact_info = mock_contact_info();
        let low_prio_bundle1 = Bundle{
            source: 0,
            destinations: vec![1],
            priority: 1,
            size: 4_000.0,
            expiration: 500.0
        };
        let low_prio_bundle2 = Bundle{
            source: 0,
            destinations: vec![1],
            priority: 1,
            size: 2_000.0,
            expiration: 300.0
        };
        //Add first two bundles (should suceed)
        assert!(mgr.schedule_tx(&contact_info, 0.0, &low_prio_bundle1).is_some(),"First bundle should have been added.");
        assert!(mgr.schedule_tx(&contact_info, 0.0, &low_prio_bundle2).is_some(),"Second bundle should have been added.");
        
        let high_prio_bundle = Bundle{
            source: 0,
            destinations: vec![1],
            priority: 2,
            size: 7_000.0,
            expiration: 300.0
        };
        let result = mgr.schedule_tx(&contact_info, 0.0, &high_prio_bundle);
        assert!(result.is_some(),"High prio bundle should have been added.");
    }
    #[test]
    fn test_schedule_with_start_offset_creates_left_segment(){
        let rate_segments = vec![
            Segment{
                start: 0.0,
                end: 100.0,
                val: 1000.0
            }
        ];
        let delay_segments = vec![
            Segment{
                start: 0.0,
                end: 100.0,
                val: 1.0
            }
        ];
        let mut mgr = setup_manager(rate_segments, delay_segments);
        let contatc_info = mock_contact_info();

        let bundle = Bundle{
            source: 0,
            destinations: vec![1],
            priority: 1,
            size: 3000.0,
            expiration: 200.0
        };
        let result = mgr.schedule_tx(&contatc_info, 20.0, &bundle);
        assert!(result.is_some(),"Bundle should have been added.");
    }


    #[test]
    fn test_successful_parsing(){
        let input_script = vec![
            "rate", "0.0", "100.0", "1000.0",
            "rate", "100.0", "150.0", "300.0",
            "delay", "0.0", "150.0", "1.0"

        ];
        let lexer: MockLexer = MockLexer::new(input_script); 
        let mut boxed_lexer = Box::new(lexer);
        let state = PSegmentationManager::parse(&mut *boxed_lexer);
        match state {
            ParsingState::Finished(mgr) => {
                assert_eq!(mgr.rate_intervals.len(),2,"Should have two rate_intervals.");

                //First rate segment checks
                assert_eq!(mgr.rate_intervals[0].start, 0.0);
                assert_eq!(mgr.rate_intervals[0].end, 100.0);
                assert_eq!(mgr.rate_intervals[0].val, 1000.0);

                //Second rate segment checks
                assert_eq!(mgr.rate_intervals[1].start, 100.0);
                assert_eq!(mgr.rate_intervals[1].end, 150.0);
                assert_eq!(mgr.rate_intervals[1].val, 300.0);

                //Delay checks
                assert_eq!(mgr.delay_intervals[0].start, 0.0);
                assert_eq!(mgr.delay_intervals[0].end, 150.0);
                assert_eq!(mgr.delay_intervals[0].val, 1.0);
                assert_eq!(mgr.delay_intervals.len(), 1);

            }
            ParsingState::EOF => {
                panic!("Parsing failed too early (EOF)");
            }
            ParsingState::Error(msg) => {
                panic!("Parsing failed: {}",msg);
            }
        }
    }

    #[test]
    fn test_parsing_fail(){
        let input_script = vec![
            "rate", "0.0", "100.0", "NOT_A_NUMBER",
            "delay", "0.0", "100.0", "1.0"
        ];
        let lexer = MockLexer::new(input_script);
        let mut boxed_lexer = Box::new(lexer);
        let result = PSegmentationManager::parse(&mut *boxed_lexer);
        if let ParsingState::Error(_) = result {
            assert!(true); //What we expect
        }
        else{
            panic!("Parsing should have failed on NOT_A_NUMBER.");
        }
        }
    }

