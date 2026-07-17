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