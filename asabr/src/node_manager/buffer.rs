extern crate alloc;
use crate::parse_transparent;
use alloc::collections::BTreeMap;
use core::convert::TryFrom;

use crate::{
    bundle::Bundle,
    errors::ASABRError,
    node_manager::NodeManager,
    types::{Date, NodeID, TimeInterval, Volume},
};

/// Manages the temporal memory buffer of a node
#[derive(Debug, Clone)]
pub struct BufferManager {
    memory: BTreeMap<Date, Volume>,
    capacity: Volume,
}

impl BufferManager {
    pub fn new(capacity: Volume) -> Self {
        let mut memory = BTreeMap::new();
        // Initialize t=0 with an empty buffer to simplify edge cases
        memory.insert(0, 0);
        Self { memory, capacity }
    }

    /// Checks if the buffer can accommodate size between start and end.
    fn check_capacity(&self, start: Date, end: Date, size: Volume) -> bool {
        if end < start {
            return false;
        }
        if end == start {
            return true;
        }

        // Find the active volume at or immediately preceding start
        let current_vol = self
            .memory
            .range(..=start)
            .next_back()
            .map(|(_, &v)| v)
            .unwrap_or(0);

        if current_vol + size > self.capacity {
            return false;
        }

        // Check all volume changes strictly between start and end
        for (_, &vol) in self.memory.range((start + 1)..end) {
            if vol + size > self.capacity {
                return false;
            }
        }

        true
    }
}

impl NodeManager for BufferManager {
    fn accept(&self, bundle: &Bundle, reception: TimeInterval, _sender: NodeID) -> bool {
        self.check_capacity(reception.start, reception.end, bundle.size)
    }

    fn process_delay(
        &self,
        _bundle: &Bundle,
        reception: TimeInterval,
        _sender: NodeID,
        _next_vertex: NodeID,
    ) -> Date {
        reception.end
    }

    fn dry_run_retention(
        &self,
        bundle: &Bundle,
        reception: TimeInterval,
        _sender: NodeID,
        transmission: TimeInterval,
        _next_vertex: NodeID,
    ) -> bool {
        self.check_capacity(reception.start, transmission.end, bundle.size)
    }

    fn dry_run_multi(
        &self,
        bundle: &Bundle,
        reception: TimeInterval,
        _sender: NodeID,
        transmissions: &[(TimeInterval, NodeID)],
    ) -> Option<usize> {
        let mut release = reception.end;
        for (interval, _) in transmissions {
            if interval.end > release {
                release = interval.end;
            }
        }

        if self.check_capacity(reception.start, release, bundle.size) {
            Some(transmissions.len())
        } else {
            None
        }
    }

    fn commit(
        &mut self,
        bundle: &Bundle,
        reception: TimeInterval,
        _sender: NodeID,
        transmissions: &[(TimeInterval, NodeID)],
    ) -> Result<(), ASABRError> {
        let mut release = reception.end;
        for (interval, _) in transmissions {
            if interval.end > release {
                release = interval.end;
            }
        }

        // Ensure a boundary exists at the exact reception statr
        let vol_at_start = self
            .memory
            .range(..=reception.start)
            .next_back()
            .map(|(_, &v)| v)
            .unwrap_or(0);
        self.memory.entry(reception.start).or_insert(vol_at_start);

        // Ensure a boundary exists at the exact release time
        let vol_at_release = self
            .memory
            .range(..=release)
            .next_back()
            .map(|(_, &v)| v)
            .unwrap_or(0);
        self.memory.entry(release).or_insert(vol_at_release);

        // Add the bundle size to all intervals strictly within [start, release)
        for (_, vol) in self.memory.range_mut(reception.start..release) {
            *vol += bundle.size;
        }

        Ok(())
    }
}

impl TryFrom<Volume> for BufferManager {
    type Error = &'static str;

    fn try_from(capacity: Volume) -> Result<Self, Self::Error> {
        Ok(BufferManager::new(capacity))
    }
}

parse_transparent!(BufferManager, Volume);

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    fn interval(start: Date, end: Date) -> TimeInterval {
        TimeInterval { start, end }
    }

    fn bundle(size: Volume) -> Bundle {
        Bundle {
            priority: 0,
            size,
            expiration: 1000,
        }
    }

    fn manager(capacity: Volume, memory: &[(Date, Volume)]) -> BufferManager {
        let mut btree = BTreeMap::new();
        for &(k, v) in memory {
            btree.insert(k, v);
        }
        BufferManager {
            memory: btree,
            capacity,
        }
    }

    fn sender() -> NodeID {
        1usize.into()
    }

    fn next() -> NodeID {
        2usize.into()
    }

    fn transmission(start: Date, end: Date, next_node: NodeID) -> (TimeInterval, NodeID) {
        (interval(start, end), next_node)
    }

    #[test]
    fn check_capacity_accepts_empty_memory_and_zero_length_intervals() {
        let manager = BufferManager::new(10);
        assert!(manager.check_capacity(0, 10, 10));
        assert!(manager.check_capacity(5, 5, 1000));
        assert!(!manager.check_capacity(6, 5, 1));
    }

    #[test]
    fn check_capacity_handles_one_tuple_boundaries() {
        let manager = manager(10, &[(0, 0), (5, 4)]);
        assert!(manager.check_capacity(0, 5, 10));
        assert!(manager.check_capacity(0, 6, 6));
        assert!(!manager.check_capacity(0, 6, 7));
        assert!(manager.check_capacity(5, 10, 6));
        assert!(!manager.check_capacity(5, 10, 7));
        assert!(manager.check_capacity(20, 30, 6));
    }

    #[test]
    fn check_capacity_handles_several_tuples_and_start_positions() {
        let manager = manager(10, &[(0, 0), (10, 3), (20, 8), (30, 1)]);
        assert!(manager.check_capacity(0, 10, 10));
        assert!(manager.check_capacity(10, 20, 7));
        assert!(!manager.check_capacity(10, 20, 8));
        assert!(manager.check_capacity(20, 30, 2));
        assert!(!manager.check_capacity(20, 30, 3));
        assert!(manager.check_capacity(15, 18, 7));
        assert!(!manager.check_capacity(15, 25, 3));
        assert!(manager.check_capacity(40, 50, 9));
    }

    #[test]
    fn check_capacity_handles_end_positions() {
        let manager = manager(10, &[(0, 0), (10, 2), (20, 5), (30, 9)]);
        assert!(manager.check_capacity(0, 9, 10));
        assert!(manager.check_capacity(0, 10, 10));
        assert!(manager.check_capacity(0, 25, 5));
        assert!(!manager.check_capacity(0, 35, 2));
        assert!(manager.check_capacity(20, 30, 5));
        assert!(!manager.check_capacity(20, 31, 2));
    }

    #[test]
    fn check_capacity_counts_internal_tuples_and_capacity_edges() {
        let manager = manager(10, &[(0, 0), (10, 3), (20, 6), (30, 4), (40, 1)]);
        assert!(manager.check_capacity(11, 19, 7));
        assert!(manager.check_capacity(11, 21, 4));
        assert!(manager.check_capacity(11, 39, 4));
        assert!(!manager.check_capacity(11, 39, 5));
        assert!(manager.check_capacity(20, 30, 4));
        assert!(!manager.check_capacity(20, 30, 5));
    }

    #[test]
    fn check_capacity_uses_current_volume_before_start() {
        let manager = manager(10, &[(0, 0), (10, 2), (20, 9), (30, 1)]);
        assert!(manager.check_capacity(25, 28, 1));
        assert!(!manager.check_capacity(25, 28, 2));
        assert!(manager.check_capacity(35, 45, 9));
        assert!(!manager.check_capacity(35, 45, 10));
    }

    #[test]
    fn accept_matches_check_capacity_for_success_and_capacity_failures() {
        let manager = manager(10, &[(0, 0), (10, 3), (20, 8), (30, 1)]);
        let small = bundle(2);
        let large = bundle(3);

        for reception in [
            interval(0, 5),
            interval(10, 20),
            interval(15, 25),
            interval(20, 30),
            interval(40, 50),
        ] {
            assert_eq!(
                manager.accept(&small, reception, sender()),
                manager.check_capacity(reception.start, reception.end, small.size)
            );
            assert_eq!(
                manager.accept(&large, reception, sender()),
                manager.check_capacity(reception.start, reception.end, large.size)
            );
        }
    }

    #[test]
    fn accept_matches_check_capacity_with_empty_memory() {
        let manager = BufferManager::new(5);
        let fits = bundle(5);
        let too_large = bundle(6);

        assert_eq!(
            manager.accept(&fits, interval(0, 10), sender()),
            manager.check_capacity(0, 10, fits.size)
        );
        assert_eq!(
            manager.accept(&too_large, interval(0, 10), sender()),
            manager.check_capacity(0, 10, too_large.size)
        );
    }

    #[test]
    fn dry_run_retention_uses_reception_start_and_transmission_end() {
        let manager = manager(10, &[(0, 0), (10, 1), (20, 9), (30, 1)]);
        let b = bundle(1);
        assert!(manager.dry_run_retention(
            &b,
            interval(5, 15),
            sender(),
            interval(100, 20),
            next(),
        ));
        assert!(!manager.dry_run_retention(
            &bundle(2),
            interval(5, 15),
            sender(),
            interval(100, 21),
            next(),
        ));
    }

    #[test]
    fn dry_run_multi_handles_no_one_and_several_transmissions() {
        let manager = manager(10, &[(0, 0), (10, 4), (20, 7), (30, 1)]);

        assert_eq!(
            manager.dry_run_multi(&bundle(6), interval(0, 10), sender(), &[]),
            Some(0)
        );
        assert_eq!(
            manager.dry_run_multi(
                &bundle(3),
                interval(10, 11),
                sender(),
                &[transmission(12, 20, next())]
            ),
            Some(1)
        );
        assert_eq!(
            manager.dry_run_multi(
                &bundle(3),
                interval(10, 11),
                sender(),
                &[
                    transmission(18, 21, next()),
                    transmission(12, 25, 3usize.into()),
                    transmission(14, 19, 4usize.into()),
                ]
            ),
            Some(3)
        );
    }

    #[test]
    fn dry_run_multi_returns_none_when_capacity_is_insufficient() {
        let manager = manager(10, &[(0, 0), (10, 4), (20, 8), (30, 1)]);
        assert_eq!(
            manager.dry_run_multi(
                &bundle(3),
                interval(10, 11),
                sender(),
                &[transmission(12, 25, next())]
            ),
            None
        );
    }

    #[test]
    fn commit_updates_empty_memory_and_first_tuple() {
        let mut manager = BufferManager::new(10);
        manager
            .commit(
                &bundle(3),
                interval(5, 7),
                sender(),
                &[transmission(10, 20, next())],
            )
            .unwrap();

        let expected: Vec<(Date, Volume)> = vec![(0, 0), (5, 3), (20, 0)];
        let actual: Vec<(Date, Volume)> = manager.memory.into_iter().collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn commit_inserts_start_and_release_boundaries() {
        let mut manager = manager(10, &[(0, 0), (10, 2), (20, 4), (30, 1)]);
        manager
            .commit(
                &bundle(3),
                interval(15, 18),
                sender(),
                &[transmission(22, 25, next())],
            )
            .unwrap();

        let expected: Vec<(Date, Volume)> =
            vec![(0, 0), (10, 2), (15, 5), (20, 7), (25, 4), (30, 1)];
        let actual: Vec<(Date, Volume)> = manager.memory.into_iter().collect();
        assert_eq!(actual, expected);
    }
}
