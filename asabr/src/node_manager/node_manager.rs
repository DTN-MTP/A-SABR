extern crate alloc;

use alloc::vec::Vec;

use crate::{
    bundle::Bundle,
    errors::ASABRError,
    node_manager::NodeManager,
    types::{Date, NodeID, TimeInterval, Volume},
};

#[derive(Debug, Clone)]
pub struct StorageNodeManager {
    /// Memory occupation:
    /// (date, volume occupied from this date until the next change)
    memory: Vec<(Date, Volume)>,
    capacity: Volume,
}

impl StorageNodeManager {
    pub fn new(capacity: Volume) -> Self {
        Self {
            memory: Vec::new(),
            capacity,
        }
    }

    fn check_start_end(
        &self,
        start: Date,
        end: Date,
        size: Volume,
    ) -> bool {
        if end < start {
            return false;
        }

        if end == start {
            return true;
        }

        let mut idx = 0;
        let mut current_volume = 0;

        while idx < self.memory.len() && self.memory[idx].0 < start {
            current_volume = self.memory[idx].1;
            idx += 1;
        }

        if current_volume + size > self.capacity {
            return false;
        }

        while idx < self.memory.len() && self.memory[idx].0 < end {
            if self.memory[idx].1 + size > self.capacity {
                return false;
            }

            idx += 1;
        }

        true
    }
}

impl NodeManager for StorageNodeManager {
    fn accept(
        &self,
        bundle: &Bundle,
        reception: TimeInterval,
        _sender: NodeID,
    ) -> bool {
        self.check_start_end(
            reception.start,
            reception.end,
            bundle.size,
        )
    }

    fn delay( // Retourne la date à laquelle le bundle peut être retransmis.
        &self,
        _bundle: &Bundle,
        reception: TimeInterval,
        _sender: NodeID,
        _next: NodeID,
    ) -> Date {
        reception.end
    }

    fn dry_run_retention( //on vérifie pour toutes les intervalle de temps avant le release si la taille du bundle ajoutée à la somme des réservations existantes dépasse la capacité du nœud. Si c'est le cas, on retourne false, sinon true.
        &self,
        bundle: &Bundle,
        reception: TimeInterval,
        _sender: NodeID,
        transmission: TimeInterval,
        _next: NodeID,
    ) -> bool {
        self.check_start_end(
            reception.start,
            transmission.end,
            bundle.size,
        )
    }

    fn dry_run_multi(
        &self,
        bundle: &Bundle,
        reception: TimeInterval,
        _sender: NodeID,
        transmissions: &[(TimeInterval, NodeID)],
    ) -> Option<usize> {
        let release = transmissions
            .iter()
            .map(|(interval, _)| interval.end)
            .max()
            .unwrap_or(reception.end);

        if self.check_start_end(
            reception.start,
            release,
            bundle.size,
        ) {
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
        let release = transmissions
            .iter()
            .map(|(interval, _)| interval.end)
            .max()
            .unwrap_or(reception.end);

        if !self.check_start_end(
            reception.start,
            release,
            bundle.size,
        ) {
            return Err(ASABRError::ScheduleError(
                "insufficient capacity",
            ));
        }

        let mut idx = 0;

        while idx < self.memory.len()
            && self.memory[idx].0 < reception.start
        {
            idx += 1;
        }

        if idx == self.memory.len()
            || self.memory[idx].0 != reception.start
        {
            let volume = if idx == 0 {
                0
            } else {
                self.memory[idx - 1].1
            };

            self.memory.insert(
                idx,
                (reception.start, volume),
            );
        }

        let mut idx = 0;

        while idx < self.memory.len()
            && self.memory[idx].0 < release
        {
            idx += 1;
        }

        if idx == self.memory.len()
            || self.memory[idx].0 != release
        {
            let volume = if idx == 0 {
                0
            } else {
                self.memory[idx - 1].1
            };

            self.memory.insert(
                idx,
                (release, volume),
            );
        }

        for idx in 0..self.memory.len() {
            if self.memory[idx].0 >= reception.start
                && self.memory[idx].0 < release
            {
                self.memory[idx].1 += bundle.size;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn interval(start: Date, end: Date) -> TimeInterval {
        TimeInterval { start, end }
    }

    fn bundle(size: Volume) -> Bundle {
        Bundle {
            source: 0usize.into(),
            priority: 0,
            size,
            expiration: 1000,
        }
    }

    fn manager(
        capacity: Volume,
        memory: &[(Date, Volume)],
    ) -> StorageNodeManager {
        StorageNodeManager {
            memory: memory.to_vec(),
            capacity,
        }
    }

    fn sender() -> NodeID {
        1usize.into()
    }

    fn next() -> NodeID {
        2usize.into()
    }

    fn transmission(
        start: Date,
        end: Date,
        next: NodeID,
    ) -> (TimeInterval, NodeID) {
        (interval(start, end), next)
    }

    #[test]
    fn check_start_end_accepts_empty_memory_and_zero_length_intervals(
    ) {
        let manager = StorageNodeManager::new(10);

        assert!(manager.check_start_end(0, 10, 10));
        assert!(manager.check_start_end(5, 5, 1000));
        assert!(!manager.check_start_end(6, 5, 1));
    }

    #[test]
    fn check_start_end_handles_one_tuple_boundaries() {
        let manager = manager(10, &[(5, 4)]);

        assert!(manager.check_start_end(0, 5, 10));
        assert!(manager.check_start_end(0, 6, 6));
        assert!(!manager.check_start_end(0, 6, 7));
        assert!(manager.check_start_end(5, 10, 6));
        assert!(!manager.check_start_end(5, 10, 7));
        assert!(manager.check_start_end(20, 30, 6));
    }

    #[test]
    fn check_start_end_handles_several_tuples_and_start_positions() {
        let manager = manager(10, &[(10, 3), (20, 8), (30, 1)]);

        assert!(manager.check_start_end(0, 10, 10));
        assert!(manager.check_start_end(10, 20, 7));
        assert!(!manager.check_start_end(10, 20, 8));
        assert!(manager.check_start_end(20, 30, 2));
        assert!(!manager.check_start_end(20, 30, 3));
        assert!(manager.check_start_end(15, 18, 7));
        assert!(!manager.check_start_end(15, 25, 3));
        assert!(manager.check_start_end(40, 50, 9));
    }

    #[test]
    fn check_start_end_handles_end_positions() {
        let manager = manager(10, &[(10, 2), (20, 5), (30, 9)]);

        assert!(manager.check_start_end(0, 9, 10));
        assert!(manager.check_start_end(0, 10, 10));
        assert!(manager.check_start_end(0, 25, 5));
        assert!(!manager.check_start_end(0, 35, 2));
        assert!(manager.check_start_end(20, 30, 5));
        assert!(!manager.check_start_end(20, 31, 2));
    }

    #[test]
    fn check_start_end_counts_internal_tuples_and_capacity_edges() {
        let manager = manager(10, &[(10, 3), (20, 6), (30, 4), (40, 1)]);

        assert!(manager.check_start_end(11, 19, 7));
        assert!(manager.check_start_end(11, 21, 4));
        assert!(manager.check_start_end(11, 39, 4));
        assert!(!manager.check_start_end(11, 39, 5));
        assert!(manager.check_start_end(20, 30, 4));
        assert!(!manager.check_start_end(20, 30, 5));
    }

    #[test]
    fn check_start_end_uses_current_volume_before_start() {
        let manager = manager(10, &[(10, 2), (20, 9), (30, 1)]);

        assert!(manager.check_start_end(25, 28, 1));
        assert!(!manager.check_start_end(25, 28, 2));
        assert!(manager.check_start_end(35, 45, 9));
        assert!(!manager.check_start_end(35, 45, 10));
    }

    #[test]
    fn accept_matches_check_start_end_for_success_and_capacity_failures(
    ) {
        let manager = manager(10, &[(10, 3), (20, 8), (30, 1)]);
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
                manager.check_start_end(
                    reception.start,
                    reception.end,
                    small.size
                )
            );
            assert_eq!(
                manager.accept(&large, reception, sender()),
                manager.check_start_end(
                    reception.start,
                    reception.end,
                    large.size
                )
            );
        }
    }

    #[test]
    fn accept_matches_check_start_end_with_empty_memory() {
        let manager = StorageNodeManager::new(5);
        let fits = bundle(5);
        let too_large = bundle(6);

        assert_eq!(
            manager.accept(&fits, interval(0, 10), sender()),
            manager.check_start_end(0, 10, fits.size)
        );
        assert_eq!(
            manager.accept(&too_large, interval(0, 10), sender()),
            manager.check_start_end(0, 10, too_large.size)
        );
    }

    #[test]
    fn dry_run_retention_uses_reception_start_and_transmission_end() {
        let manager = manager(10, &[(10, 1), (20, 9), (30, 1)]);
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
    fn dry_run_retention_covers_boundary_and_capacity_edges() {
        let manager = manager(10, &[(10, 4), (20, 6), (30, 10)]);

        assert!(manager.dry_run_retention(
            &bundle(4),
            interval(20, 21),
            sender(),
            interval(25, 30),
            next(),
        ));
        assert!(!manager.dry_run_retention(
            &bundle(1),
            interval(20, 21),
            sender(),
            interval(25, 31),
            next(),
        ));
        assert!(manager.dry_run_retention(
            &bundle(100),
            interval(25, 25),
            sender(),
            interval(0, 25),
            next(),
        ));
        assert!(!manager.dry_run_retention(
            &bundle(1),
            interval(31, 32),
            sender(),
            interval(0, 29),
            next(),
        ));
    }

    #[test]
    fn dry_run_multi_handles_no_one_and_several_transmissions() {
        let manager = manager(10, &[(10, 4), (20, 7), (30, 1)]);

        assert_eq!(
            manager.dry_run_multi(
                &bundle(6),
                interval(0, 10),
                sender(),
                &[]
            ),
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
    fn dry_run_multi_release_is_maximum_transmission_end_not_start(
    ) {
        let manager = manager(10, &[(10, 2), (20, 9), (30, 1)]);
        let transmissions = [
            transmission(100, 18, next()),
            transmission(15, 19, 3usize.into()),
            transmission(12, 21, 4usize.into()),
        ];

        assert_eq!(
            manager.dry_run_multi(
                &bundle(1),
                interval(5, 8),
                sender(),
                &transmissions
            ),
            Some(3)
        );
        assert_eq!(
            manager.dry_run_multi(
                &bundle(2),
                interval(5, 8),
                sender(),
                &transmissions
            ),
            None
        );
    }

    #[test]
    fn dry_run_multi_returns_none_when_capacity_is_insufficient() {
        let manager = manager(10, &[(10, 4), (20, 8), (30, 1)]);

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
        let mut manager = StorageNodeManager::new(10);

        manager
            .commit(
                &bundle(3),
                interval(5, 7),
                sender(),
                &[transmission(10, 20, next())],
            )
            .unwrap();

        assert_eq!(manager.memory, vec![(5, 3), (20, 0)]);
    }

    #[test]
    fn commit_inserts_start_and_release_boundaries() {
        let mut manager = manager(10, &[(10, 2), (20, 4), (30, 1)]);

        manager
            .commit(
                &bundle(3),
                interval(15, 18),
                sender(),
                &[transmission(22, 25, next())],
            )
            .unwrap();

        assert_eq!(
            manager.memory,
            vec![(10, 2), (15, 5), (20, 7), (25, 4), (30, 1)]
        );
    }

    #[test]
    fn commit_reuses_existing_start_and_release_boundaries() {
        let mut manager = manager(10, &[(10, 2), (20, 4), (30, 1)]);

        manager
            .commit(
                &bundle(3),
                interval(10, 12),
                sender(),
                &[transmission(22, 30, next())],
            )
            .unwrap();

        assert_eq!(manager.memory, vec![(10, 5), (20, 7), (30, 1)]);
    }

    #[test]
    fn commit_adds_volume_only_until_release_and_keeps_after_release(
    ) {
        let mut manager = manager(10, &[(0, 1), (10, 2), (20, 3), (30, 4)]);

        manager
            .commit(
                &bundle(5),
                interval(5, 6),
                sender(),
                &[transmission(50, 25, next())],
            )
            .unwrap();

        assert_eq!(
            manager.memory,
            vec![(0, 1), (5, 6), (10, 7), (20, 8), (25, 3), (30, 4)]
        );
    }

    #[test]
    fn commit_updates_multiple_tuples_and_last_tuple() {
        let mut manager = manager(10, &[(10, 1), (20, 2), (30, 3)]);

        manager
            .commit(
                &bundle(4),
                interval(15, 16),
                sender(),
                &[transmission(18, 40, next())],
            )
            .unwrap();

        assert_eq!(
            manager.memory,
            vec![(10, 1), (15, 5), (20, 6), (30, 7), (40, 3)]
        );
    }

    #[test]
    fn commit_inserts_at_beginning_and_end() {
        let mut manager = manager(10, &[(10, 2), (20, 3)]);

        manager
            .commit(
                &bundle(4),
                interval(0, 1),
                sender(),
                &[transmission(30, 40, next())],
            )
            .unwrap();

        assert_eq!(
            manager.memory,
            vec![(0, 4), (10, 6), (20, 7), (40, 3)]
        );
    }

    #[test]
    fn commit_failure_returns_error_and_does_not_modify_memory() {
        let original = vec![(10, 5), (20, 9), (30, 1)];
        let mut manager = manager(10, &original);

        let result = manager.commit(
            &bundle(2),
            interval(15, 16),
            sender(),
            &[transmission(50, 25, next())],
        );

        assert!(matches!(
            result,
            Err(ASABRError::ScheduleError("insufficient capacity"))
        ));
        assert_eq!(manager.memory, original);
    }

    #[test]
    fn commit_exact_boundary_cases_and_spanning_reservations() {
        let mut manager = manager(12, &[(10, 2), (20, 5), (30, 6), (40, 2)]);

        manager
            .commit(
                &bundle(6),
                interval(20, 20),
                sender(),
                &[
                    transmission(99, 30, next()),
                    transmission(15, 40, 3usize.into()),
                ],
            )
            .unwrap();

        assert_eq!(
            manager.memory,
            vec![(10, 2), (20, 11), (30, 12), (40, 2)]
        );
    }
}
