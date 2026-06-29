use crate::bundle::Bundle;
use crate::contact::ContactInfo;
use crate::types::{DataRate, Date, Duration, Volume};

pub(crate) const RATE: DataRate = 1000;
pub(crate) const DELAY: Duration = 1;
pub(crate) const C_START: Date = 0;
pub(crate) const C_END: Date = 10;
pub(crate) const TOTAL_VOL: Volume = 10000;
pub(crate) const BUDGET_P0: Volume = 2000;
pub(crate) const BUDGET_P1: Volume = 5000;
pub(crate) const BUDGET_P2: Volume = TOTAL_VOL;

extern crate alloc;

pub(crate) fn make_contact_info(start: Date, end: Date) -> ContactInfo {
    ContactInfo::new(0.into(), 1.into(), start, end)
}

pub(crate) fn make_bundle(size: Volume, priority: i8) -> Bundle {
    Bundle {
        source: 0.into(),
        priority,
        size,
        expiration: 99999,
    }
}

pub(crate) fn bp0(size: Volume) -> Bundle {
    make_bundle(size, 0)
}
pub(crate) fn bp1(size: Volume) -> Bundle {
    make_bundle(size, 1)
}
pub(crate) fn bp2(size: Volume) -> Bundle {
    make_bundle(size, 2)
}

#[macro_export]
macro_rules! generate_common_tests {
    ($manager_fn:expr, $manager_type:ty) => {
        #[test]
        fn try_init_zero_duration_rejects_any_bundle() {
            let mut manager = <$manager_type>::new(RATE, DELAY);
            let contact = $crate::contact::ContactInfo::new(0.into(), 1.into(), 5, 5);
            manager.try_init(&contact);
            assert!(
                manager.dry_run_tx(contact.into(), 5, &bp0(1)).is_none(),
                "TEST FAILED: Expected None for a contact with a duration of zero."
            );
        }

        #[test]
        fn dry_run_volume_boundary() {
            let manager = ($manager_fn)();
            let ti = $crate::types::TimeInterval{start:C_START, end:C_END};
            assert!(
                manager.dry_run_tx(ti, C_START, &bp0(TOTAL_VOL)).is_some(),
                "TEST FAILED: Expected Some at exact volume boundary."
            );
            assert!(
                manager.dry_run_tx(ti, C_START, &bp0(TOTAL_VOL + 1)).is_none(),
                "TEST FAILED: Expected None above volume boundary."
            );
        }

        #[test]
        fn dry_run_contact_timing_boundaries() {
            let manager = ($manager_fn)();
            let ti = $crate::types::TimeInterval{start:C_START, end:C_END};
            assert!(
                manager.dry_run_tx(ti, C_END + 1, &bp0(1)).is_none(),
                "TEST FAILED: Expected None when at_time is past contact end."
            );
            assert!(
                manager.dry_run_tx(ti, C_END - 0, &bp0(600)).is_none(),
                "TEST FAILED: Expected None when bundle does not fit in remaining time."
            );
        }

        #[test]
        fn dry_run_makes_same_results() {
            let manager = ($manager_fn)();
            let ti = $crate::types::TimeInterval{start:C_START, end:C_END};
            let bundle = bp0(100);
            assert_eq!(
                manager.dry_run_tx(ti, C_START, &bundle),
                manager.dry_run_tx(ti, C_START, &bundle),
                "TEST FAILED: dry_run_tx should make the same results each time."
            );
        }

        #[test]
        fn tx_data_fields_are_correct() {
            let data = ($manager_fn)()
                .dry_run_tx($crate::types::TimeInterval{start:C_START, end:C_END}, C_START, &bp0(100))
                .unwrap();
            assert_eq!(
                data.expiration,
                C_END,
                "TEST FAILED: expiration should equal C_END."
            );
            assert_eq!(
                data.rx_window.start,
                data.tx_window.start + DELAY,
                "TEST FAILED: rx_start should equal tx_start + DELAY."
            );
             assert_eq!(
                data.rx_window.end,
                data.tx_window.end + DELAY,
                "TEST FAILED: rx_end should equal tx_end + DELAY."
            );
        }

        #[test]
        fn schedule_tx_matches_dry_run_on_fresh_manager() {
            let manager_dry = ($manager_fn)();
            let mut manager_sched = ($manager_fn)();
            let ti = $crate::types::TimeInterval{start:C_START, end:C_END};
            let bundle = bp0(100);
            assert_eq!(
                manager_dry.dry_run_tx(ti, C_START, &bundle),
                manager_sched.schedule_tx(ti, C_START, &bundle),
                "TEST FAILED: schedule_tx and dry_run_tx should return identical timings on a fresh manager."
            );
        }

        #[test]
        fn single_prio_manager_ignores_priority_field() {
            let manager = ($manager_fn)();
            let ti = $crate::types::TimeInterval{start:C_START, end:C_END};
            assert_eq!(
                manager.dry_run_tx(ti, C_START, &bp0(100)),
                manager.dry_run_tx(ti, C_START, &bp1(100)),
                "TEST FAILED: Single-priority manager should return identical timings for p0 and p1."
            );
            assert_eq!(
                manager.dry_run_tx(ti, C_START, &bp0(100)),
                manager.dry_run_tx(ti, C_START, &bp2(100)),
                "TEST FAILED: Single-priority manager should return identical timings for p0 and p2."
            );
        }
    };
}

#[macro_export]
macro_rules! generate_auto_update_tests {
    ($manager_fn:expr, $p_manager_fn:expr) => {
        #[test]
        fn schedule_tx_saturation() {
            let mut manager = ($manager_fn)();
            let ti = $crate::types::TimeInterval {
                start: C_START,
                end: C_END,
            };
            for i in 0..10 {
                assert!(
                    manager.schedule_tx(ti, C_START, &bp0(1000)).is_some(),
                    "TEST FAILED: Expected Some on schedule {} of 10.",
                    i + 1
                );
            }
            assert!(
                manager.schedule_tx(ti, C_START, &bp0(1)).is_none(),
                "TEST FAILED: Expected None after volume is fully saturated."
            );
        }

        #[test]
        fn priority_cascade_and_isolation() {
            let mut manager = ($p_manager_fn)();
            let ti = $crate::types::TimeInterval {
                start: C_START,
                end: C_END,
            };
            assert!(
                manager.schedule_tx(ti, C_START, &bp2(5000)).is_some(),
                "TEST FAILED: Expected Some scheduling p2 bundle."
            );
            assert!(
                manager.dry_run_tx(ti, C_START, &bp0(5000)).is_some(),
                "TEST FAILED: Expected Some for p0 bundle within remaining p0 budget."
            );
            assert!(
                manager.dry_run_tx(ti, C_START, &bp0(5001)).is_none(),
                "TEST FAILED: Expected None for p0 bundle exceeding remaining p0 budget (cascade)."
            );
            assert!(
                manager
                    .dry_run_tx(ti, C_START, &bp2(TOTAL_VOL - 5000))
                    .is_some(),
                "TEST FAILED: Expected Some for p2 bundle within remaining global volume."
            );
        }

        #[test]
        fn mid_prio_cascades_down_but_not_up() {
            let mut manager = ($p_manager_fn)();
            let ti = $crate::types::TimeInterval {
                start: C_START,
                end: C_END,
            };
            assert!(
                manager.schedule_tx(ti, C_START, &bp1(5000)).is_some(),
                "TEST FAILED: Expected Some scheduling p1 bundle."
            );
            assert!(
                manager.dry_run_tx(ti, C_START, &bp0(5001)).is_none(),
                "TEST FAILED: Expected None for p0 -> p1 cascade should have consumed p0 budget."
            );
            assert!(
                manager.dry_run_tx(ti, C_START, &bp2(TOTAL_VOL)).is_some(),
                "TEST FAILED: Expected Some for p2 -> p1 should not cascade upward."
            );
        }
    };
}

#[macro_export]
macro_rules! generate_budget_tests {
    ($pb_manager_fn:expr) => {
        #[test]
        fn budget_hard_limits_per_priority() {
            let manager = ($pb_manager_fn)();
            let ti = $crate::types::TimeInterval {
                start: C_START,
                end: C_END,
            };
            assert!(
                manager.dry_run_tx(ti, C_START, &bp0(BUDGET_P0)).is_some(),
                "TEST FAILED: Expected Some for p0 bundle at exact budget."
            );
            assert!(
                manager
                    .dry_run_tx(ti, C_START, &bp0(BUDGET_P0 + 1))
                    .is_none(),
                "TEST FAILED: Expected None for p0 bundle exceeding budget."
            );
            assert!(
                manager.dry_run_tx(ti, C_START, &bp1(BUDGET_P1)).is_some(),
                "TEST FAILED: Expected Some for p1 bundle at exact budget."
            );
            assert!(
                manager
                    .dry_run_tx(ti, C_START, &bp1(BUDGET_P1 + 1))
                    .is_none(),
                "TEST FAILED: Expected None for p1 bundle exceeding budget."
            );
        }
    };
}

#[macro_export]
macro_rules! generate_budget_auto_update_tests {
    ($pb_manager_fn:expr) => {
        #[test]
        fn budget_priorities_are_independent() {
            let mut manager = ($pb_manager_fn)();
            let ti = $crate::types::TimeInterval {
                start: C_START,
                end: C_END,
            };
            assert!(
                manager.schedule_tx(ti, C_START, &bp0(BUDGET_P0)).is_some(),
                "TEST FAILED: Expected Some scheduling p0 bundle up to its budget."
            );
            assert!(
                manager.dry_run_tx(ti, C_START, &bp0(1)).is_none(),
                "TEST FAILED: Expected None -> p0 budget should be exhausted."
            );
            assert!(
                manager.dry_run_tx(ti, C_START, &bp2(100)).is_some(),
                "TEST FAILED: Expected Some -> p2 budget should be independent of p0."
            );
            assert!(
                manager.dry_run_tx(ti, C_START, &bp1(1)).is_some(),
                "TEST FAILED: Expected Some -> p1 budget should be independent of p0."
            );
        }
    };
}
