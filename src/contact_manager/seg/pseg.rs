use std::cmp::max;

#[cfg(feature = "first_depleted")]
use crate::types::Volume;
use crate::{bundle::Bundle, contact::ContactInfo, contact_manager::{ContactManager, ContactManagerTxData, seg::{Segment, SegmentationManager}}, types::{DataRate, Date, Duration, Priority}};

#[derive(Debug)]
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

    fn get_tx_end(&self, mut at_time: Date, mut volume: Volume) -> Option<Date> {
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

        if volume > 0.0 {
            return None;
        }
        Some(tx_end)
    }
}

impl ContactManager for PSegmentationManager {

    fn dry_run_tx(&self, _contact_data: &ContactInfo, at_time: Date,bundle: &Bundle,) -> Option<ContactManagerTxData> {


        let mut tx_start = at_time;
        let mut tx_end_opt: Option<Date> = None;


        for seg in &self.booking {
            // Is the seg valid ?
            if seg.end < at_time || bundle.priority <= seg.val {
                tx_end_opt = None;
                continue;
            }
            match tx_end_opt {
                // Try to pursue the build process
                Some(tx_end) => {
                    // the seg is valid, check if this is the last one to consider
                    if tx_end < seg.end {
                        let delay = SegmentationManager::get_delay(tx_end, &self.delay_intervals);
                        return Some(ContactManagerTxData {
                             tx_start,
                             tx_end,
                             delay,
                             expiration: seg.end,
                             arrival: tx_end + delay,
                        });
                    }

                },
                // Start the build process
                None =>  {
                    tx_start = Date::max(seg.start, at_time);
                    if let Some(tx_end) = self.get_tx_end(tx_start, bundle.size) {
                        tx_end_opt = Some(tx_end);
                    };
                },
            }
        }
        None
    }

   fn schedule_tx(&mut self,contact_data: &ContactInfo,at_time: Date,bundle: &Bundle,) -> Option<ContactManagerTxData> {
        todo!()
    }


    #[cfg(feature = "first_depleted")]
    fn get_original_volume(&self) -> Volume {
        todo!()
    }

    fn try_init(&mut self,contact_data: &ContactInfo) -> bool {
        todo!()
    }
}