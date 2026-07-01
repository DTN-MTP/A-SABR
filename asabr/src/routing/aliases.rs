extern crate alloc;

use super::spsn::Spsn;
#[allow(unused_imports)]
use super::cgr::Cgr;
#[cfg(feature = "contact_suppression")]
use crate::pathfinding::limiting_contact::Suppressor;
use crate::{
    contact_manager::ContactManager,
    contact_plan::ContactPlan,
    distance::{hop::Hop, sabr::SABR},
    errors::ASABRError,
    node_manager::NodeManager,
    pathfinding::{
        Pathfinding,
        destination::Destination,
        dijkstra_impl::{ContactParenting, HybridParenting, NodeParenting},
    },
    route_storage::{cache::TreeCache, table::RoutingTable},
    routing::volcgr::VolCgr,
};
use alloc::boxed::Box;

pub type SpsnHybridParenting<'id, const PRIO_COUNT: usize, NM, CM, D> =
    Spsn<'id, PRIO_COUNT, NM, CM, HybridParenting<'id, SABR, NM, CM>, TreeCache<'id, NM, CM>, D>;

pub type SpsnNodeParenting<'id, const PRIO_COUNT: usize, NM, CM, D> =
    Spsn<'id, PRIO_COUNT, NM, CM, NodeParenting<'id, SABR>, TreeCache<'id, NM, CM>, D>;

pub type SpsnContactParenting<'id, const PRIO_COUNT: usize, NM, CM, D> =
    Spsn<'id, PRIO_COUNT, NM, CM, ContactParenting<'id, SABR, NM, CM>, TreeCache<'id, NM, CM>, D>;

pub type VolCgrHybridParenting<'id, NM, CM, D> =
    VolCgr<'id, NM, CM, HybridParenting<'id, NM, CM, SABR>, RoutingTable<'id, NM, CM, SABR>, D>;

pub type VolCgrNodeParenting<'id, NM, CM, D> =
    VolCgr<'id, NM, CM, NodeParenting<'id, SABR>, RoutingTable<'id, NM, CM, SABR>, D>;

pub type VolCgrContactParenting<'id, NM, CM, D> =
    VolCgr<'id, NM, CM, ContactParenting<'id, NM, CM, SABR>, RoutingTable<'id, NM, CM, SABR>, D>;

#[cfg(feature = "contact_suppression")]
pub type CgrSupressorHybridParenting<'id, NM, CM, D> = Cgr<
    'id,
    NM,
    CM,
    Suppressor<'id, HybridParenting<'id, SABR, NM, CM>, NM, CM>,
    RoutingTable<'id, NM, CM, SABR>,
    D,
>;

#[cfg(feature = "contact_suppression")]
pub type CgrSupressorNodeParenting<'id, NM, CM, D> = Cgr<
    'id,
    NM,
    CM,
    Suppressor<'id, NodeParenting<'id, SABR>, NM, CM>,
    RoutingTable<'id, NM, CM, SABR>,
    D,
>;

#[cfg(feature = "contact_suppression")]
pub type CgrSupressorContactParenting<'id, NM, CM, D> = Cgr<
    'id,
    NM,
    CM,
    Suppressor<'id, ContactParenting<'id, NM, CM, SABR>, NM, CM>,
    RoutingTable<'id, NM, CM, SABR>,
    D,
>;

pub type SpsnHybridParentingHop<'id, const PRIO_COUNT: usize, NM, CM, D> =
    Spsn<'id, PRIO_COUNT, NM, CM, HybridParenting<'id, Hop, NM, CM>, TreeCache<'id, NM, CM>, D>;

pub type SpsnNodeParentingHop<'id, const PRIO_COUNT: usize, NM, CM, D> =
    Spsn<'id, PRIO_COUNT, NM, CM, NodeParenting<'id, Hop>, TreeCache<'id, NM, CM>, D>;

pub type SpsnContactParentingHop<'id, const PRIO_COUNT: usize, NM, CM, D> =
    Spsn<'id, PRIO_COUNT, NM, CM, ContactParenting<'id, Hop, NM, CM>, TreeCache<'id, NM, CM>, D>;

pub type VolCgrHybridParentingHop<'id, NM, CM, D> =
    VolCgr<'id, NM, CM, HybridParenting<'id, Hop, NM, CM>, RoutingTable<'id, NM, CM, Hop>, D>;

pub type VolCgrNodeParentingHop<'id, NM, CM, D> =
    VolCgr<'id, NM, CM, NodeParenting<'id, Hop>, RoutingTable<'id, NM, CM, Hop>, D>;

pub type VolCgrContactParentingHop<'id, NM, CM, D> =
    VolCgr<'id, NM, CM, ContactParenting<'id, NM, CM, Hop>, RoutingTable<'id, NM, CM, Hop>, D>;

#[cfg(feature = "contact_suppression")]
pub type CgrSupressorHybridParentingHop<'id, NM, CM, D> = Cgr<
    'id,
    NM,
    CM,
    Suppressor<'id, NM, CM, HybridParenting<'id, Hop, NM, CM>>,
    RoutingTable<'id, NM, CM, Hop>,
    D,
>;

#[cfg(feature = "contact_suppression")]
pub type CgrSuppressorNodeParentingHop<'id, NM, CM, D> = Cgr<
    'id,
    NM,
    CM,
    Suppressor<'id, NM, CM, NodeParenting<'id, Hop>>,
    RoutingTable<'id, NM, CM, Hop>,
    D,
>;

#[cfg(feature = "contact_suppression")]
pub type CgrSupressorContactParentingHop<'id, NM, CM, D> = Cgr<
    'id,
    NM,
    CM,
    Suppressor<'id, NM, CM, ContactParenting<'id, NM, CM, Hop>>,
    RoutingTable<'id, NM, CM, Hop>,
    D,
>;

// macro_rules! register_cgr_router {
//     ($router:ident, $router_name:literal, $test_name_variable:ident, $contact_plan:ident) => {
//         if $test_name_variable == $router_name {
//             let routing_table = Rc::new(RefCell::new(RoutingTable::new()));

//             return Ok(Box::new($router::<NM, CM>::new(
//                 $contact_plan,
//                 routing_table,
//             )?));
//         }
//     };
// }

// macro_rules! register_spsn_router {
//     ($router:ident, $router_name:literal, $test_name_variable:ident, $contact_plan:ident, $check_size:ident, $check_priority:ident, $max_entries:ident) => {
//         if $test_name_variable == $router_name {
//             let cache = Rc::new(RefCell::new(TreeCache::new(
//                 $check_size,
//                 $check_priority,
//                 $max_entries,
//             )));

//             return Ok(Box::new($router::<NM, CM>::new(
//                 $contact_plan,
//                 cache,
//                 $check_priority,
//             )?));
//         }
//     };
// }
#[derive(Clone)]
pub struct SpsnOptions {
    pub check_size: bool,
    pub check_priority: bool,
    pub max_entries: usize,
}

pub fn build_generic_router<
    'id,
    NM: NodeManager + 'static,
    CM: ContactManager + 'static,
    D: Destination<'id>,
>(
    _router_type: &str,
    _contact_plan: ContactPlan<NM, CM>,
    _spsn_options: Option<SpsnOptions>,
) -> Result<Box<dyn Pathfinding<'id, NM, CM, D>>, ASABRError> {
    todo!()
}
//     if let Some(options) = spsn_options {
//         let check_size = options.check_size;
//         let check_priority = options.check_priority;
//         let max_entries = options.max_entries;

//         register_spsn_router!(
//             SpsnNodeParenting,
//             "SpsnNodeParenting",
//             router_type,
//             contact_plan,
//             check_size,
//             check_priority,
//             max_entries
//         );

//         register_spsn_router!(
//             SpsnNodeParentingHop,
//             "SpsnNodeParentingHop",
//             router_type,
//             contact_plan,
//             check_size,
//             check_priority,
//             max_entries
//         );

//         register_spsn_router!(
//             SpsnHybridParenting,
//             "SpsnHybridParenting",
//             router_type,
//             contact_plan,
//             check_size,
//             check_priority,
//             max_entries
//         );

//         register_spsn_router!(
//             SpsnHybridParentingHop,
//             "SpsnHybridParentingHop",
//             router_type,
//             contact_plan,
//             check_size,
//             check_priority,
//             max_entries
//         );

//         register_spsn_router!(
//             SpsnContactParenting,
//             "SpsnContactParenting",
//             router_type,
//             contact_plan,
//             check_size,
//             check_priority,
//             max_entries
//         );

//         register_spsn_router!(
//             SpsnContactParentingHop,
//             "SpsnContactParentingHop",
//             router_type,
//             contact_plan,
//             check_size,
//             check_priority,
//             max_entries
//         );
//     }

//     register_cgr_router!(
//         VolCgrNodeParenting,
//         "VolCgrNodeParenting",
//         router_type,
//         contact_plan
//     );

//     register_cgr_router!(
//         VolCgrHybridParenting,
//         "VolCgrHybridParenting",
//         router_type,
//         contact_plan
//     );

//     register_cgr_router!(
//         VolCgrHybridParentingHop,
//         "VolCgrHybridParentingHop",
//         router_type,
//         contact_plan
//     );

//     register_cgr_router!(
//         VolCgrNodeParentingHop,
//         "VolCgrNodeParentingHop",
//         router_type,
//         contact_plan
//     );

//     register_cgr_router!(
//         VolCgrContactParenting,
//         "VolCgrContactParenting",
//         router_type,
//         contact_plan
//     );

//     register_cgr_router!(
//         VolCgrContactParentingHop,
//         "VolCgrContactParentingHop",
//         router_type,
//         contact_plan
//     );

//     #[cfg(feature = "contact_suppression")]
//     register_cgr_router!(
//         CgrFirstEndingHybridParentingHop,
//         "CgrFirstEndingHybridParentingHop",
//         router_type,
//         contact_plan
//     );

//     #[cfg(feature = "contact_suppression")]
//     register_cgr_router!(
//         CgrFirstEndingHybridParenting,
//         "CgrFirstEndingHybridParenting",
//         router_type,
//         contact_plan
//     );

//     #[cfg(feature = "contact_suppression")]
//     register_cgr_router!(
//         CgrFirstEndingNodeParentingHop,
//         "CgrFirstEndingNodeParentingHop",
//         router_type,
//         contact_plan
//     );

//     #[cfg(feature = "contact_suppression")]
//     register_cgr_router!(
//         CgrFirstEndingNodeParenting,
//         "CgrFirstEndingNodeParenting",
//         router_type,
//         contact_plan
//     );

//     #[cfg(feature = "contact_suppression")]
//     register_cgr_router!(
//         CgrFirstEndingContactParentingHop,
//         "CgrFirstEndingContactParentingHop",
//         router_type,
//         contact_plan
//     );

//     #[cfg(feature = "contact_suppression")]
//     register_cgr_router!(
//         CgrFirstEndingContactParenting,
//         "CgrFirstEndingContactParenting",
//         router_type,
//         contact_plan
//     );

//     #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
//     register_cgr_router!(
//         CgrFirstDepletedHybridParentingHop,
//         "CgrFirstDepletedHybridParentingHop",
//         router_type,
//         contact_plan
//     );

//     #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
//     register_cgr_router!(
//         CgrFirstDepletedHybridParenting,
//         "CgrFirstDepletedHybridParenting",
//         router_type,
//         contact_plan
//     );

//     #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
//     register_cgr_router!(
//         CgrFirstDepletedNodeParentingHop,
//         "CgrFirstDepletedNodeParentingHop",
//         router_type,
//         contact_plan
//     );

//     #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
//     register_cgr_router!(
//         CgrFirstDepletedNodeParenting,
//         "CgrFirstDepletedNodeParenting",
//         router_type,
//         contact_plan
//     );

//     #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
//     register_cgr_router!(
//         CgrFirstDepletedContactParentingHop,
//         "CgrFirstDepletedContactParentingHop",
//         router_type,
//         contact_plan
//     );

//     #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
//     register_cgr_router!(
//         CgrFirstDepletedContactParenting,
//         "CgrFirstDepletedContactParenting",
//         router_type,
//         contact_plan
//     );

//     Err(ASABRError::ScheduleError(
//         "Router type is invalid! (check for typo, disabled feature, or missing options for Spsn algos)",
//     ))
// }
