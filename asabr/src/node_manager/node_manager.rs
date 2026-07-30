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


impl NodeManager for StorageNodeManager {

    fn accept(
        &self,
        bundle: &Bundle,
        reception: TimeInterval,
        _sender: NodeID,
    ) -> bool {

        let mut result = self.memory[self.memory.len() - 1].1;

        let mut idx = 0;
        while idx < self.memory.len() && self.memory[idx].0 < reception.start {
            idx+=1;
        }

        while idx < self.memory.len() && self.memory[idx].0 <= reception.end {
            
            if self.memory[idx].1 + bundle.size > self.capacity {
                return false;
            }
            idx+=1;
        }

        return true
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

    fn dry_run_retention( //on vérifie pour toutes les intervalle de temps avant le release si la taille du bundle ajoutée à la somme des réservations existantes dépasse la capacité du nœud. Si c'est le cas, on retourne false, sinon true.
        &self,
        bundle: &Bundle,
        reception: TimeInterval,
        _sender: NodeID,
        transmission: TimeInterval,
        _next: NodeID,
    ) -> bool {

        let mut result = self.memory[self.memory.len() - 1].1;

        let mut idx = 0;
        while idx < self.memory.len() && self.memory[idx].0 < reception.start {
            idx+=1;
        }

        while idx < self.memory.len() && self.memory[idx].0 <= transmission.end {
            
            if self.memory[idx].1 + bundle.size > self.capacity {
                return false;
            }
            idx+=1;
        }

        return true
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
            .map(|(interval, _)| interval.start)
            .max()
            .unwrap_or(reception.end);

        let retention = (
            reception.start,
            release,
    );

        if self.simulate_retention(
            retention.start,
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


        let mut result = self.memory[self.memory.len() - 1].1;

        let mut idx = 0;
        while idx < self.memory.len() && self.memory[idx].0 < reception.start {
            idx+=1;
        }
        
        if reception.start < self.memory[idx].0 {
            self.memory.insert(idx, (reception.start, self.memory[idx - 1].1));
        }

        while idx < self.memory.len() && transmission.end > self.memory[idx].0 {
                self.memory[idx].1 += bundle.size;
                idx+= 1;
        }

        if transmission.end < self.memory[idx].0 {
            self.memory.insert(idx, (transmission.end, self.memory[idx - 1].1 - bundle.size));
        }

    }
    
}
}