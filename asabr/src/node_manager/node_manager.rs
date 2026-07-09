use alloc::vec::Vec;

use crate::{
    bundle::Bundle,
    errors::ASABRError,
    node_manager::NodeManager,
    types::{Date, NodeID, TimeInterval, Volume},
};

#[derive(Debug, Clone)]
pub struct StorageNodeManager {
    reservations: Vec<(TimeInterval, Volume)>,
    capacity: Volume,
}
impl StorageNodeManager {
    pub fn new(capacity: Volume) -> Self {
        Self {
            reservations: Vec::new(),
            capacity,
        }
    }
        fn can_reserve(
            &self,
            interval: TimeInterval,
            volume: Volume,
            ) -> bool {

            let mut events: Vec<(Date, Volume)> = Vec::new();

            for (reserved, used) in &self.reservations {

                if reserved.end <= interval.start || reserved.start >= interval.end {
                    continue;
                }

                let start = reserved.start.max(interval.start);
                let end = reserved.end.min(interval.end);

                events.push((start, *used));
                events.push((end, -*used));
            }

            events.push((interval.start, volume));
            events.push((interval.end, -volume));

            events.sort_by_key(|e| e.0);

            let mut current = 0;

            for (_, delta) in events {
                current += delta;

                if current > self.capacity {
                    return false;
                }
            }

            true
        }
}
impl NodeManager for StorageNodeManager {

    fn accept(
        &self,
        bundle: &Bundle,
        _time: TimeInterval,
        _sender: NodeID,
    ) -> bool {
        bundle.size <= self.capacity
    }

    fn delay(
        &self,
        _bundle: &Bundle,
        reception: TimeInterval,
        _sender: NodeID,
        _next: NodeID,
    ) -> Date {
        reception.end
    }

    fn dry_run_retention(
        &self,
        bundle: &Bundle,
        reception: TimeInterval,
        _sender: NodeID,
        transmission: TimeInterval,
        _next: NodeID,
    ) -> bool {

        let retention = TimeInterval {
            start: reception.start,
            end: transmission.start,
        };

        self.can_reserve(retention, bundle.size)
    }

    fn dry_run_multi(
        &self,
        bundle: &Bundle,
        reception: TimeInterval,
        _sender: NodeID,
        transmissions: &[(TimeInterval, NodeID)],
    ) -> Option<usize> {

        let end = transmissions
            .iter()
            .map(|(t, _)| t.start)
            .max()
            .unwrap_or(reception.end);

        let retention = TimeInterval {
            start: reception.start,
            end,
        };

        if self.can_reserve(retention, bundle.size) {
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

        let end = transmissions
            .iter()
            .map(|(t, _)| t.start)
            .max()
            .unwrap_or(reception.end);

        self.reservations.push((
            TimeInterval {
                start: reception.start,
                end,
            },
            bundle.size,
        ));

        Ok(())
    }

}