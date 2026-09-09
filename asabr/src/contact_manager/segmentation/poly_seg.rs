#[cfg(feature = "first_depleted")]
use crate::types::Volume;
use crate::{
    bundle::Bundle,
    contact::ContactInfo,
    contact_manager::{
        segmentation::Segment, ContactManager, ContactManagerTxData,
    },
    errors::ASABRError,
    poly::Polynome,
    types::{Date, Duration, TimeInterval},
};

extern crate alloc;
#[allow(unused_imports)]
use alloc::{vec, vec::Vec};

/// Segmentation manager based on a polynomial-modeled flow rate.
#[derive(Debug, Clone)]
pub struct PolySegManager<const N: usize> {
    /// The remaining free time slots for this contact.
    free_intervals: Vec<Segment<()>>,
    /// Propagation/processing delay intervals.
    delay_intervals: Vec<Segment<Duration>>,
    /// The polynomial evaluating the link capacity
    pub polynome: Polynome<N>,
    #[cfg(feature = "first_depleted")]
    /// The total volume at initialization.
    original_volume: Volume,
}

impl<const N: usize> PolySegManager<N> {
    pub fn new(
        polynome: Polynome<N>,
        delay_intervals: Vec<Segment<Duration>>,
    ) -> Self {
        Self {
            free_intervals: Vec::new(),
            delay_intervals,
            polynome,
            #[cfg(feature = "first_depleted")]
            original_volume: 0,
        }
    }
}

impl<const N: usize> ContactManager for PolySegManager<N> {
    fn dry_run_tx(
        &self,
        _contact_data: TimeInterval,
        at_time: Date,
        bundle: &Bundle,
    ) -> Option<ContactManagerTxData> {
        let mut tx_start: Date;

        for free_seg in &self.free_intervals {
            if free_seg.end < at_time {
                continue;
            }
            tx_start = Date::max(free_seg.start, at_time);
            
            let Some(tx_end) = self.polynome.find_end_bundle(
                free_seg.start,
                free_seg.end,
                tx_start,
                bundle.size,
            ) else {
                continue;
            };

            let (d_start, d_end) = super::get_delays(tx_start, tx_end, &self.delay_intervals);
            return Some(ContactManagerTxData {
                send: TimeInterval {
                    start: tx_start,
                    end: tx_end,
                },
                recv: TimeInterval {
                    start: tx_start + d_start,
                    end: tx_end + d_end,
                },
            });
        }
        None
    }

    fn schedule_tx(
        &mut self,
        _contact_data: TimeInterval,
        tx_data: ContactManagerTxData,
        _bundle: &Bundle,
    ) -> Result<(), ASABRError> {
        let tx_start = tx_data.send.start;
        let tx_end = tx_data.send.end;

        let index = self
            .free_intervals
            .binary_search_by_key(&tx_start, |seg| seg.start)
            .unwrap_or_else(|err| err - 1);

        let interval = self
            .free_intervals
            .get_mut(index)
            .ok_or(ASABRError::ScheduleError("Illegal tx_data"))?;

        if interval.start != tx_start {
            let old_end = interval.end;
            interval.end = tx_start;
            if interval.end != tx_end {
                self.free_intervals.insert(
                    index + 1,
                    Segment {
                        start: tx_end,
                        end: old_end,
                        val: (),
                    },
                )
            }
        } else {
            interval.start = tx_end;
        }
        Ok(())
    }

    fn try_init(&mut self, contact_data: &ContactInfo) -> bool {
        // Initialization of the full free interval
        self.free_intervals.clear();
        self.free_intervals.push(Segment {
            start: contact_data.start,
            end: contact_data.end,
            val: (),
        });

        #[cfg(feature = "first_depleted")]
        {
            let vol_start = self.polynome.evaluate_exact_integral(contact_data.start);
            let vol_end = self.polynome.evaluate_exact_integral(contact_data.end);
            self.original_volume = vol_end - vol_start;
        }

        true 
    }

    #[cfg(feature = "first_depleted")]
    fn get_original_volume(&self) -> Volume {
        self.original_volume
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bundle::Bundle,
        contact::ContactInfo,
        contact_manager::ContactManager,
        poly::Polynome,
        types::TimeInterval,
    };

    const TEST_N: usize = 26;

    #[test]
    fn test_poly_segmentation_simple() {
        // Creation of a constant polynomial (flow rate of 100 units per second)
        let mut coeffs = [0; TEST_N];
        coeffs[0] = 100; 
        let poly = Polynome::new(coeffs, 0);

        // Creation of a constant 4-second delay across the entire contact.
        let delay_segments = vec![Segment {
            start: 0,
            end: 200,
            val: 4,
        }];

        // Manager initialization
        let mut manager = PolySegManager::new(poly, delay_segments);
        let contact_info = ContactInfo::new(0.into(), 1.into(), 0, 200);
        assert!(manager.try_init(&contact_info), "Initialization failed");

        // Creation of a packet of size 1000
        let bundle = Bundle {
            priority: 1,
            size: 1000,
            expiration: 1000,
        };

        // dry_run test: the package should take 10 seconds (1000 / 100)
        let dry_run_res = manager.dry_run_tx(
            TimeInterval { start: 0, end: 200 },
            0, // sent at T=0
            &bundle,
        );

        assert!(dry_run_res.is_some(), "The bundle should be able to pass");
        let tx_data = dry_run_res.unwrap();
        
        // Verification of time windows
        assert_eq!(tx_data.send.start, 0);
        assert_eq!(tx_data.send.end, 10);
        assert_eq!(tx_data.recv.start, 4); // start + delay
        assert_eq!(tx_data.recv.end, 14); // end + delay

        // Schedule test: reserving the time
        let schedule_res = manager.schedule_tx(
            TimeInterval { start: 0, end: 200 },
            tx_data,
            &bundle,
        );
        assert!(schedule_res.is_ok(), "Scheduling failed");

        // Check of remaining free intervals: [10, 200] must remain.
        assert_eq!(manager.free_intervals.len(), 1);
        assert_eq!(manager.free_intervals[0].start, 10);
        assert_eq!(manager.free_intervals[0].end, 200);
    }

    #[test]
    fn test_poly_segmentation_parabola() {
        // Manual initialization of the polynomial P(t) = 10 + 3*t^2, offset = 0
        let mut coeffs = [0; TEST_N];
        coeffs[0] = 10;
        coeffs[1] = 0;
        coeffs[2] = 3;
        let poly = Polynome::new(coeffs, 0);

        // Manager initialization (network delay of 2 units)
        let delay_segments = vec![Segment { start: 0, end: 100, val: 2 }];
        let mut manager = PolySegManager::new(poly, delay_segments);

        let contact_info = ContactInfo::new(0.into(), 1.into(), 0, 100);
        assert!(manager.try_init(&contact_info));

        // Bundle creation (analytically calculated size: 104)
        let bundle = Bundle { priority: 1, size: 104, expiration: 1000 };

        // Capacity prediction execution
        let dry_run_res = manager.dry_run_tx(
            TimeInterval { start: 0, end: 100 },
            0, // Start of transmission at T=0
            &bundle,
        );

        // Verification of the integration engine and dichotomy
        assert!(dry_run_res.is_some(), "The bundle should pass");
        let tx_data = dry_run_res.unwrap();
        
        assert_eq!(tx_data.send.start, 0);
        assert_eq!(tx_data.send.end, 4, "The dichotomy did not find the exact root of the polynomial");
        assert_eq!(tx_data.recv.start, 2); // 0 + delay(2)
        assert_eq!(tx_data.recv.end, 6);   // 4 + delay(2)
    }
}