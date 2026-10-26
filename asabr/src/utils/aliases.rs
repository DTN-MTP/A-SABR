use crate::pathfinding::top_level::volcgr::VolCgr;
#[cfg(feature = "contact_suppression")]
use crate::pathfinding::{limiting_contact::Suppressor, top_level::cgr::Cgr};
use crate::{
    pathfinding::{
        ContactParenting, DestAll, HybridParenting, NodeParenting, top_level::spsn::Spsn,
    },
    route_storage::{cache::TreeCache, table::RoutingTable},
};

/// SPSN router using SABR distance and hybrid parenting.
pub type SpsnHybridParenting<'id, const PRIO_COUNT: usize, NM, CM, D, CST> =
    Spsn<'id, PRIO_COUNT, NM, CM, HybridParenting<'id, CST, NM, CM>, TreeCache<'id, NM, CM>, D>;

/// SPSN router using SABR distance and node parenting.
pub type SpsnNodeParenting<'id, const PRIO_COUNT: usize, NM, CM, D, CST> =
    Spsn<'id, PRIO_COUNT, NM, CM, NodeParenting<'id, CST>, TreeCache<'id, NM, CM>, D>;

/// SPSN router using SABR distance and contact parenting.
pub type SpsnContactParenting<'id, const PRIO_COUNT: usize, NM, CM, D, CST> = Spsn<
    'id,
    PRIO_COUNT,
    NM,
    CM,
    ContactParenting<'id, NM, CM, CST, DestAll>,
    TreeCache<'id, NM, CM>,
    D,
>;

/// VolCGR router using SABR distance and hybrid parenting.
pub type VolCgrHybridParenting<'id, NM, CM, D, CST> =
    VolCgr<'id, RoutingTable<'id, D, NM, CM>, HybridParenting<'id, CST, NM, CM>, NM, CM, D>;

/// VolCGR router using SABR distance and node parenting.
pub type VolCgrNodeParenting<'id, NM, CM, D, CST> =
    VolCgr<'id, RoutingTable<'id, D, NM, CM>, NodeParenting<'id, CST>, NM, CM, D>;

/// VolCGR router using SABR distance and contact parenting.
pub type VolCgrContactParenting<'id, NM, CM, D, CST> =
    VolCgr<'id, RoutingTable<'id, D, NM, CM>, ContactParenting<'id, NM, CM, CST, D>, NM, CM, D>;

#[cfg(feature = "contact_suppression")]
pub type CgrSupressorHybridParenting<'id, NM, CM, D, CST> = Cgr<
    'id,
    NM,
    CM,
    Suppressor<'id, HybridParenting<'id, CST, NM, CM>, NM, CM>,
    RoutingTable<'id, D, NM, CM>,
    D,
>;

#[cfg(feature = "contact_suppression")]
pub type CgrSupressorNodeParenting<'id, NM, CM, D, CST> = Cgr<
    'id,
    NM,
    CM,
    Suppressor<'id, NodeParenting<'id, CST>, NM, CM>,
    RoutingTable<'id, D, NM, CM>,
    D,
>;

#[cfg(feature = "contact_suppression")]
pub type CgrSupressorContactParenting<'id, NM, CM, D, CST> = Cgr<
    'id,
    NM,
    CM,
    Suppressor<'id, ContactParenting<'id, NM, CM, CST, D>, NM, CM>,
    RoutingTable<'id, D, NM, CM>,
    D,
>;
