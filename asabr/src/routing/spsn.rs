use crate::route_storage::{Cached, Guarded};

/// A structure representing the Shortest Path with Safety Nodes (SPSN) algorithm.
///
/// This struct handles routing logic and pathfinding, utilizing stored routes
/// and ensuring that the routing process adheres to specified safety and priority constraints.
///
/// # Type Parameters
/// - prio_count: the number of priority to handle in the guard. Set to 1 to ignore priority.
/// - `NM`: A type that implements the `NodeManager` trait, responsible for managing the
///   network's nodes and their interactions.
/// - `CM`: A type that implements the `ContactManager` trait, handling contact points and
///   communication schedules within the network.
/// - `P`: A type that implements the `Pathfinding<NM, CM>` trait, responsible for computing optimal paths.
pub type Spsn<'id, const PRIO_COUNT: usize, NM, CM, P, S, D> =
    Guarded<'id, PRIO_COUNT, Cached<'id, S, P, NM, CM, D>, D, NM, CM>;
