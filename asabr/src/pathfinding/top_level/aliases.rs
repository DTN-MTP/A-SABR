extern crate alloc;
use alloc::boxed::Box;

#[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
use crate::pathfinding::limiting_contact::had_less_volume_than;
#[cfg(feature = "contact_suppression")]
use crate::pathfinding::limiting_contact::{Suppressor, ends_earlier_than};

use crate::{
    contact_manager::ContactManager,
    contact_plan::ContactPlan,
    distance::{hop::Hop, sabr::SABR},
    errors::ASABRError,
    multigraph::{Multigraph, RoutableNodeRef},
    node_manager::NodeManager,
    pathfinding::{
        Pathfinding,
        dijkstra_impl::{ContactParenting, HybridParenting, NodeParenting},
    },
    route_storage::{cache::TreeCache, table::RoutingTable},
};
#[allow(unused_imports)]
use {super::cgr::Cgr, super::spsn::Spsn, super::volcgr::VolCgr};

/// SPSN router using SABR distance and hybrid parenting.
pub type SpsnHybridParenting<'id, const PRIO_COUNT: usize, NM, CM, D> =
    Spsn<'id, PRIO_COUNT, NM, CM, HybridParenting<'id, SABR, NM, CM>, TreeCache<'id, NM, CM>, D>;

/// SPSN router using SABR distance and node parenting.
pub type SpsnNodeParenting<'id, const PRIO_COUNT: usize, NM, CM, D> =
    Spsn<'id, PRIO_COUNT, NM, CM, NodeParenting<'id, SABR>, TreeCache<'id, NM, CM>, D>;

/// SPSN router using SABR distance and contact parenting.
pub type SpsnContactParenting<'id, const PRIO_COUNT: usize, NM, CM, D> =
    Spsn<'id, PRIO_COUNT, NM, CM, ContactParenting<'id, NM, CM, SABR>, TreeCache<'id, NM, CM>, D>;

/// VolCGR router using SABR distance and hybrid parenting.
pub type VolCgrHybridParenting<'id, NM, CM, D> =
    VolCgr<'id, RoutingTable<'id, D, NM, CM>, HybridParenting<'id, SABR, NM, CM>, NM, CM, D>;

/// VolCGR router using SABR distance and node parenting.
pub type VolCgrNodeParenting<'id, NM, CM, D> =
    VolCgr<'id, RoutingTable<'id, D, NM, CM>, NodeParenting<'id, SABR>, NM, CM, D>;

/// VolCGR router using SABR distance and contact parenting.
pub type VolCgrContactParenting<'id, NM, CM, D> =
    VolCgr<'id, RoutingTable<'id, D, NM, CM>, ContactParenting<'id, NM, CM, SABR>, NM, CM, D>;

#[cfg(feature = "contact_suppression")]
pub type CgrSupressorHybridParenting<'id, NM, CM, D> = Cgr<
    'id,
    NM,
    CM,
    Suppressor<'id, HybridParenting<'id, SABR, NM, CM>, NM, CM>,
    RoutingTable<'id, D, NM, CM>,
    D,
>;

#[cfg(feature = "contact_suppression")]
pub type CgrSupressorNodeParenting<'id, NM, CM, D> = Cgr<
    'id,
    NM,
    CM,
    Suppressor<'id, NodeParenting<'id, SABR>, NM, CM>,
    RoutingTable<'id, D, NM, CM>,
    D,
>;

#[cfg(feature = "contact_suppression")]
pub type CgrSupressorContactParenting<'id, NM, CM, D> = Cgr<
    'id,
    NM,
    CM,
    Suppressor<'id, ContactParenting<'id, NM, CM, SABR>, NM, CM>,
    RoutingTable<'id, D, NM, CM>,
    D,
>;

/// SPSN router using hop distance and hybrid parenting.
pub type SpsnHybridParentingHop<'id, const PRIO_COUNT: usize, NM, CM, D> =
    Spsn<'id, PRIO_COUNT, NM, CM, HybridParenting<'id, Hop, NM, CM>, TreeCache<'id, NM, CM>, D>;

/// SPSN router using hop distance and node parenting.
pub type SpsnNodeParentingHop<'id, const PRIO_COUNT: usize, NM, CM, D> =
    Spsn<'id, PRIO_COUNT, NM, CM, NodeParenting<'id, Hop>, TreeCache<'id, NM, CM>, D>;

/// SPSN router using hop distance and contact parenting.
pub type SpsnContactParentingHop<'id, const PRIO_COUNT: usize, NM, CM, D> =
    Spsn<'id, PRIO_COUNT, NM, CM, ContactParenting<'id, NM, CM, Hop>, TreeCache<'id, NM, CM>, D>;

/// VolCGR router using hop distance and hybrid parenting.
pub type VolCgrHybridParentingHop<'id, NM, CM, D> =
    VolCgr<'id, RoutingTable<'id, D, NM, CM>, HybridParenting<'id, Hop, NM, CM>, NM, CM, D>;

/// VolCGR router using hop distance and node parenting.
pub type VolCgrNodeParentingHop<'id, NM, CM, D> =
    VolCgr<'id, RoutingTable<'id, D, NM, CM>, NodeParenting<'id, Hop>, NM, CM, D>;

/// VolCGR router using hop distance and contact parenting.
pub type VolCgrContactParentingHop<'id, NM, CM, D> =
    VolCgr<'id, RoutingTable<'id, D, NM, CM>, ContactParenting<'id, NM, CM, Hop>, NM, CM, D>;

#[cfg(feature = "contact_suppression")]
pub type CgrSupressorHybridParentingHop<'id, NM, CM, D> = Cgr<
    'id,
    NM,
    CM,
    Suppressor<'id, HybridParenting<'id, Hop, NM, CM>, NM, CM>,
    RoutingTable<'id, D, NM, CM>,
    D,
>;

#[cfg(feature = "contact_suppression")]
pub type CgrSupressorNodeParentingHop<'id, NM, CM, D> = Cgr<
    'id,
    NM,
    CM,
    Suppressor<'id, NodeParenting<'id, Hop>, NM, CM>,
    RoutingTable<'id, D, NM, CM>,
    D,
>;

#[cfg(feature = "contact_suppression")]
pub type CgrSupressorContactParentingHop<'id, NM, CM, D> = Cgr<
    'id,
    NM,
    CM,
    Suppressor<'id, ContactParenting<'id, NM, CM, Hop>, NM, CM>,
    RoutingTable<'id, D, NM, CM>,
    D,
>;

/// Options controlling SPSN route selection.
#[derive(Clone)]
pub struct SpsnOptions {
    /// Whether bundle size is checked.
    pub check_size: bool,
    /// Whether bundle priority is checked.
    pub check_priority: bool,
    /// Maximum number of entries to keep.
    pub max_entries: usize,
}

/// Intended for tests / benchmarking where you deal with a bunch of router types, not production code
/// Initialise the correct router directly where possible
/// # Safety
/// unsafe because it return a unguarded graph. see `Multigraph::new_unguarded` for more information
#[allow(clippy::type_complexity)]
pub unsafe fn build_generic_router<
    'id,
    const PRIO_COUNT: usize,
    NM: NodeManager + 'static,
    CM: ContactManager + 'static,
>(
    router_type: &str,
    contact_plan: ContactPlan<NM, CM>,
) -> Result<
    (
        Multigraph<'id, NM, CM>,
        Box<dyn Pathfinding<'id, NM, CM, RoutableNodeRef<'id>> + 'id>,
    ),
    ASABRError,
> {
    let multigraph = unsafe { Multigraph::new_unguarded(contact_plan) }?;
    let router = match router_type {
        "SpsnNodeParenting" => Box::new(SpsnNodeParenting::<PRIO_COUNT, NM, CM, _>::new(
            (&multigraph, (10, ())).into(),
        )) as Box<dyn Pathfinding<'id, NM, CM, _> + 'id>,
        "SpsnNodeParentingHop" => Box::new(SpsnNodeParentingHop::<PRIO_COUNT, NM, CM, _>::new(
            (&multigraph, (10, ())).into(),
        )),
        "SpsnHybridParenting" => Box::new(SpsnHybridParenting::<PRIO_COUNT, NM, CM, _>::new(
            (&multigraph, (10, ())).into(),
        )),
        "SpsnHybridParentingHop" => Box::new(SpsnHybridParentingHop::<PRIO_COUNT, NM, CM, _>::new(
            (&multigraph, (10, ())).into(),
        )),
        "SpsnContactParenting" => Box::new(SpsnContactParenting::<PRIO_COUNT, NM, CM, _>::new(
            (&multigraph, (10, ())).into(),
        )),
        "SpsnContactParentingHop" => Box::new(
            SpsnContactParentingHop::<PRIO_COUNT, NM, CM, _>::new((&multigraph, (10, ())).into()),
        ),
        "VolCgrNodeParenting" => Box::new(VolCgrNodeParenting::new(
            RoutingTable::new(),
            NodeParenting::new(),
        )),
        "VolCgrNodeParentingHop" => Box::new(VolCgrNodeParentingHop::new(
            RoutingTable::new(),
            NodeParenting::new(),
        )),
        "VolCgrHybridParenting" => Box::new(VolCgrHybridParenting::new(
            RoutingTable::new(),
            HybridParenting::new(),
        )),
        "VolCgrHybridParentingHop" => Box::new(VolCgrHybridParentingHop::new(
            RoutingTable::new(),
            HybridParenting::new(),
        )),
        "VolCgrContactParenting" => Box::new(VolCgrContactParenting::new(
            RoutingTable::new(),
            ContactParenting::new(),
        )),
        "VolCgrContactParentingHop" => Box::new(VolCgrContactParentingHop::new(
            RoutingTable::new(),
            ContactParenting::new(),
        )),
        #[cfg(feature = "contact_suppression")]
        "CgrFirstEndingHybridParenting" => Box::new(CgrSupressorHybridParenting::new(
            Suppressor::new(HybridParenting::new(), ends_earlier_than, &multigraph),
            RoutingTable::new(),
            &multigraph,
        )),
        #[cfg(feature = "contact_suppression")]
        "CgrFirstEndingHybridParentingHop" => Box::new(CgrSupressorHybridParentingHop::new(
            Suppressor::new(HybridParenting::new(), ends_earlier_than, &multigraph),
            RoutingTable::new(),
            &multigraph,
        )),
        #[cfg(feature = "contact_suppression")]
        "CgrFirstEndingContactParenting" => Box::new(CgrSupressorContactParenting::new(
            Suppressor::new(ContactParenting::new(), ends_earlier_than, &multigraph),
            RoutingTable::new(),
            &multigraph,
        )),
        #[cfg(feature = "contact_suppression")]
        "CgrFirstEndingContactParentingHop" => Box::new(CgrSupressorContactParentingHop::new(
            Suppressor::new(ContactParenting::new(), ends_earlier_than, &multigraph),
            RoutingTable::new(),
            &multigraph,
        )),
        #[cfg(feature = "contact_suppression")]
        "CgrFirstEndingNodeParenting" => Box::new(CgrSupressorNodeParenting::new(
            Suppressor::new(NodeParenting::new(), ends_earlier_than, &multigraph),
            RoutingTable::new(),
            &multigraph,
        )),
        #[cfg(feature = "contact_suppression")]
        "CgrFirstEndingNodeParentingHop" => Box::new(CgrSupressorNodeParentingHop::new(
            Suppressor::new(NodeParenting::new(), ends_earlier_than, &multigraph),
            RoutingTable::new(),
            &multigraph,
        )),
        #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
        "CgrFirstDepletedHybridParenting" => Box::new(CgrSupressorHybridParenting::new(
            Suppressor::new(HybridParenting::new(), had_less_volume_than, &multigraph),
            RoutingTable::new(),
            &multigraph,
        )),
        #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
        "CgrFirstDepletedHybridParentingHop" => Box::new(CgrSupressorHybridParentingHop::new(
            Suppressor::new(HybridParenting::new(), had_less_volume_than, &multigraph),
            RoutingTable::new(),
            &multigraph,
        )),
        #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
        "CgrFirstDepletedContactParenting" => Box::new(CgrSupressorContactParenting::new(
            Suppressor::new(ContactParenting::new(), had_less_volume_than, &multigraph),
            RoutingTable::new(),
            &multigraph,
        )),
        #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
        "CgrFirstDepletedContactParentingHop" => Box::new(CgrSupressorContactParentingHop::new(
            Suppressor::new(ContactParenting::new(), had_less_volume_than, &multigraph),
            RoutingTable::new(),
            &multigraph,
        )),
        #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
        "CgrFirstDepletedNodeParenting" => Box::new(CgrSupressorNodeParenting::new(
            Suppressor::new(NodeParenting::new(), had_less_volume_than, &multigraph),
            RoutingTable::new(),
            &multigraph,
        )),
        #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
        "CgrFirstDepletedNodeParentingHop" => Box::new(CgrSupressorNodeParentingHop::new(
            Suppressor::new(NodeParenting::new(), had_less_volume_than, &multigraph),
            RoutingTable::new(),
            &multigraph,
        )),

        _ => return Err(ASABRError::ContactPlanError("Not a known router type !")),
    };

    Ok((multigraph, router))
}
