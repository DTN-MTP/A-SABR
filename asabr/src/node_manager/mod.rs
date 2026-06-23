extern crate alloc;

use crate::{bundle::Bundle, errors::ASABRError, types::{Date, NodeID, TimeInterval}};
pub mod none;

/// A trait for managing and scheduling operations on nodes in a network.
///
/// The `NodeManager` trait defines methods for dry-run (simulation) and actual scheduling
/// of passing packets.
///
///
/// # Expected Guarantees
/// It is a logic error to have incoherent medhods, in particular:
///    - if dry_run_retention(...,transmition,next) return true, then dry_run_multi(...,[transmition,next]) should do so as well (with identical other parametters)
///    - if dry_run_multi(...) return true, then commit with the same parametters should suceed
///    - delay should not return a date before reception start time
/// 
/// Failing to upheld these can result in incorrect behavior of algorithms using this node manager
/// (including panic, memory leak ...), but should not result in memory unsafet
///
///
/// # Simulation:
/// - accept(bundle,time) -> return false if no such can be accepted, as an early return
/// - delay(bundle,time) -> how much delay should we wait for before trying to send the packet
/// - dry_run_retention(bundle,get_time,send_time) -> is this retention accepted
/// - dry_run_multi(bundle,get_time,&[send_time]) -> how many of these can you accept to ressend.
///
/// # Commit
/// - commit(bundle,get_time,&[send_time])
pub trait NodeManager {
    // This is important for optimisation, so no default implementation is provided
    /// Should return false if no packet of this size can be recieved by the node
    fn accept(&self, bundle: &Bundle, time: TimeInterval, sender: NodeID) -> bool;


    #[allow(unused_variables)]
    /// date at wich we can start to resend the bundle. Acount for both delay at reception and delay upon sending
    fn delay(&self, bundle: &Bundle, reception: TimeInterval, sender: NodeID, next: NodeID) -> Date{
          reception.end
    }

    #[allow(unused_variables)]
    /// Check if this retention on the node is allowed, used to compute possible routes during path-finding
    fn dry_run_retention(&self, bundle: &Bundle, reception: TimeInterval, sender: NodeID, transmition: TimeInterval, next: NodeID) -> bool;

    /// Return None if the node cannot accept the paquet, Some(n) if it can accept the paquet and retransmit it to the firsts n elements of transmitions
    /// Transmitions can be several elements (multicast) or none (destination node and no multicast)
    /// To reliably detect if this node is (one of) the destination, inspect the bundle, not transmition only.
    /// # Expected Guarantees
    /// Should be LESS restrictive than dry_run_retention but MORE restrictive than commit
    fn dry_run_multi(&self, bundle: &Bundle, reception: TimeInterval, sender: NodeID, transmitions: &[(TimeInterval,NodeID)]) -> Option<usize>;

    /// Updates ressources for this node, based on the given transmition
    /// Transmitions can be several elements (multicast) or none (destination node and no multicast)
    /// To reliably detect if this node is (one of) the destinations, inspect the bundle, not only the size of transmitio
    /// # Expected Guarantee
    /// Should not error if a previous call to dry_run_multi told us these transmition were OK.
    fn commit(&mut self, bundle: &Bundle, reception: TimeInterval, sender: NodeID, transmitions: &[(TimeInterval,NodeID)]) -> Result<(),ASABRError>;
    

}

// Implementation of `NodeManager` for dyn references.
impl<T: AsRef<dyn NodeManager> + AsMut<dyn NodeManager>> NodeManager for T {
    fn accept(&self, bundle: &Bundle, time: TimeInterval, sender: NodeID) -> bool {
        self.as_ref().accept(bundle, time, sender)
    }

    fn delay(&self, bundle: &Bundle, reception: TimeInterval, sender: NodeID, next: NodeID) -> Date {
        self.as_ref().delay(bundle, reception, sender, next)
    }

    fn dry_run_retention(&self, bundle: &Bundle, reception: TimeInterval, sender: NodeID, transmition: TimeInterval, next: NodeID) -> bool {
        self.as_ref().dry_run_retention(bundle, reception, sender, transmition, next)
    }

    fn dry_run_multi(&self, bundle: &Bundle, reception: TimeInterval, sender: NodeID, transmitions: &[(TimeInterval,NodeID)]) -> Option<usize> {
        self.as_ref().dry_run_multi(bundle, reception, sender, transmitions)
    }

    fn commit(&mut self, bundle: &Bundle, reception: TimeInterval, sender: NodeID, transmitions: &[(TimeInterval,NodeID)]) -> Result<(), ASABRError> {
        self.as_mut().commit(bundle, reception, sender, transmitions)
    }
}
/// Auto implement NodeManager for wrapper struct where element 0 is the actual node manager
#[macro_export]
macro_rules! transparent_NM {
    ($T:ty) => {
        impl NodeManager for $T {
            fn accept(&self, bundle: &Bundle, time: TimeInterval, sender: NodeID) -> bool {
                self.0.accept(bundle, time, sender)
            }
            
            fn delay(&self, bundle: &Bundle, reception: TimeInterval, sender: NodeID, next: NodeID) -> Date {
                self.0.delay(bundle, reception, sender, next)
            }

            fn dry_run_retention(&self, bundle: &Bundle, reception: TimeInterval, sender: NodeID, transmition: TimeInterval, next: NodeID) -> bool {
                self.0.dry_run_retention(bundle, reception, sender, transmition, next)
            }

            fn dry_run_multi(&self, bundle: &Bundle, reception: TimeInterval, sender: NodeID, transmitions: &[(TimeInterval,NodeID)]) -> Option<usize> {
                self.0.dry_run_multi(bundle, reception, sender, transmitions)
            }

            fn commit(&mut self, bundle: &Bundle, reception: TimeInterval, sender: NodeID, transmitions: &[(TimeInterval,NodeID)]) -> Result<(), ASABRError> {
                self.0.commit(bundle, reception, sender, transmitions)
            }
        }
    };
}
