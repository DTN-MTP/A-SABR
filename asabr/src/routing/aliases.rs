extern crate alloc;

#[cfg(feature = "first_depleted")]
use crate::pathfinding::limiting_contact::FirstDepleted;
#[cfg(feature = "contact_suppression")]
use crate::pathfinding::limiting_contact::FirstEnding;
use crate::{
    contact_manager::ContactManager,
    contact_plan::ContactPlan,
    distance::{hop::Hop, sabr::SABR},
    errors::ASABRError,
    node_manager::NodeManager,
    pathfinding::{
        contact_parenting::ContactParenting, hybrid_parenting::HybridParenting,
        node_parenting::NodeParenting,
    },
    route_storage::{cache::TreeCache, table::RoutingTable},
    routing::volcgr::VolCgr,
};
use alloc::{boxed::Box, rc::Rc};
use core::cell::RefCell;

use super::cgr::Cgr;
use super::{Router, spsn::Spsn};

pub type SpsnHybridParenting<'id, NM, CM> =
    Spsn<NM, CM, HybridParenting<'id, true, NM, CM, SABR>, TreeCache<NM, CM>>;

pub type SpsnNodeParenting<'id, NM, CM> =
    Spsn<NM, CM, NodeParenting<'id, true, NM, CM, SABR>, TreeCache<NM, CM>>;

pub type SpsnContactParenting<'id, NM, CM> =
    Spsn<NM, CM, ContactParenting<'id, true, NM, CM, SABR>, TreeCache<NM, CM>>;

pub type VolCgrHybridParenting<'id, NM, CM> =
    VolCgr<NM, CM, HybridParenting<'id, false, NM, CM, SABR>, RoutingTable<NM, CM, SABR>>;

pub type VolCgrNodeParenting<'id, NM, CM> =
    VolCgr<NM, CM, NodeParenting<'id, false, NM, CM, SABR>, RoutingTable<NM, CM, SABR>>;

pub type VolCgrContactParenting<'id, NM, CM> =
    VolCgr<NM, CM, ContactParenting<'id, false, NM, CM, SABR>, RoutingTable<NM, CM, SABR>>;

#[cfg(feature = "contact_suppression")]
pub type CgrFirstEndingHybridParenting<'id, NM, CM> = Cgr<
    NM,
    CM,
    FirstEnding<'id, NM, CM, HybridParenting<'id, false, NM, CM, SABR>>,
    RoutingTable<NM, CM, SABR>,
>;

#[cfg(feature = "first_depleted")]
pub type CgrFirstDepletedHybridParenting<'id, NM, CM> = Cgr<
    NM,
    CM,
    FirstDepleted<'id, NM, CM, HybridParenting<'id, false, NM, CM, SABR>>,
    RoutingTable<NM, CM, SABR>,
>;

#[cfg(feature = "contact_suppression")]
pub type CgrFirstEndingNodeParenting<'id, NM, CM> = Cgr<
    NM,
    CM,
    FirstEnding<'id, NM, CM, NodeParenting<'id, false, NM, CM, SABR>>,
    RoutingTable<NM, CM, SABR>,
>;

#[cfg(feature = "first_depleted")]
pub type CgrFirstDepletedNodeParenting<'id, NM, CM> = Cgr<
    NM,
    CM,
    FirstDepleted<'id, NM, CM, NodeParenting<'id, false, NM, CM, SABR>>,
    RoutingTable<NM, CM, SABR>,
>;

#[cfg(feature = "contact_suppression")]
pub type CgrFirstEndingContactParenting<'id, NM, CM> = Cgr<
    NM,
    CM,
    FirstEnding<'id, NM, CM, ContactParenting<'id, false, NM, CM, SABR>>,
    RoutingTable<NM, CM, SABR>,
>;

#[cfg(feature = "first_depleted")]
pub type CgrFirstDepletedContactParenting<'id, NM, CM> = Cgr<
    NM,
    CM,
    FirstDepleted<'id, NM, CM, ContactParenting<'id, false, NM, CM, SABR>>,
    RoutingTable<NM, CM, SABR>,
>;

pub type SpsnHybridParentingHop<'id, NM, CM> =
    Spsn<NM, CM, HybridParenting<'id, true, NM, CM, Hop>, TreeCache<NM, CM>>;

pub type SpsnNodeParentingHop<'id, NM, CM> =
    Spsn<NM, CM, NodeParenting<'id, true, NM, CM, Hop>, TreeCache<NM, CM>>;

pub type SpsnContactParentingHop<'id, NM, CM> =
    Spsn<NM, CM, ContactParenting<'id, true, NM, CM, Hop>, TreeCache<NM, CM>>;

pub type VolCgrHybridParentingHop<'id, NM, CM> =
    VolCgr<NM, CM, HybridParenting<'id, false, NM, CM, Hop>, RoutingTable<NM, CM, Hop>>;

pub type VolCgrNodeParentingHop<'id, NM, CM> =
    VolCgr<NM, CM, NodeParenting<'id, false, NM, CM, Hop>, RoutingTable<NM, CM, Hop>>;

pub type VolCgrContactParentingHop<'id, NM, CM> =
    VolCgr<NM, CM, ContactParenting<'id, false, NM, CM, Hop>, RoutingTable<NM, CM, Hop>>;

#[cfg(feature = "contact_suppression")]
pub type CgrFirstEndingHybridParentingHop<'id, NM, CM> = Cgr<
    NM,
    CM,
    FirstEnding<'id, NM, CM, HybridParenting<'id, false, NM, CM, Hop>>,
    RoutingTable<NM, CM, Hop>,
>;

#[cfg(feature = "first_depleted")]
pub type CgrFirstDepletedHybridParentingHop<'id, NM, CM> = Cgr<
    NM,
    CM,
    FirstDepleted<'id, NM, CM, HybridParenting<'id, false, NM, CM, Hop>>,
    RoutingTable<NM, CM, Hop>,
>;

#[cfg(feature = "contact_suppression")]
pub type CgrFirstEndingNodeParentingHop<'id, NM, CM> = Cgr<
    NM,
    CM,
    FirstEnding<'id, NM, CM, NodeParenting<'id, false, NM, CM, Hop>>,
    RoutingTable<NM, CM, Hop>,
>;

#[cfg(feature = "first_depleted")]
pub type CgrFirstDepletedNodeParentingHop<'id, NM, CM> = Cgr<
    NM,
    CM,
    FirstDepleted<'id, NM, CM, NodeParenting<'id, false, NM, CM, Hop>>,
    RoutingTable<NM, CM, Hop>,
>;

#[cfg(feature = "contact_suppression")]
pub type CgrFirstEndingContactParentingHop<'id, NM, CM> = Cgr<
    NM,
    CM,
    FirstEnding<'id, NM, CM, ContactParenting<'id, false, NM, CM, Hop>>,
    RoutingTable<NM, CM, Hop>,
>;

#[cfg(feature = "first_depleted")]
pub type CgrFirstDepletedContactParentingHop<'id, NM, CM> = Cgr<
    NM,
    CM,
    FirstDepleted<'id, NM, CM, ContactParenting<'id, false, NM, CM, Hop>>,
    RoutingTable<NM, CM, Hop>,
>;

macro_rules! register_cgr_router {
    ($router:ident, $router_name:literal, $test_name_variable:ident, $contact_plan:ident) => {
        if $test_name_variable == $router_name {
            let routing_table = Rc::new(RefCell::new(RoutingTable::new()));

            return Ok(Box::new($router::<NM, CM>::new(
                $contact_plan,
                routing_table,
            )?));
        }
    };
}

macro_rules! register_spsn_router {
    ($router:ident, $router_name:literal, $test_name_variable:ident, $contact_plan:ident, $check_size:ident, $check_priority:ident, $max_entries:ident) => {
        if $test_name_variable == $router_name {
            let cache = Rc::new(RefCell::new(TreeCache::new(
                $check_size,
                $check_priority,
                $max_entries,
            )));

            return Ok(Box::new($router::<NM, CM>::new(
                $contact_plan,
                cache,
                $check_priority,
            )?));
        }
    };
}
#[derive(Clone)]
pub struct SpsnOptions {
    pub check_size: bool,
    pub check_priority: bool,
    pub max_entries: usize,
}

pub fn build_generic_router<NM: NodeManager + 'static, CM: ContactManager + 'static>(
    router_type: &str,
    contact_plan: ContactPlan<NM, CM>,
    spsn_options: Option<SpsnOptions>,
) -> Result<Box<dyn Router<NM, CM>>, ASABRError> {
    if let Some(options) = spsn_options {
        let check_size = options.check_size;
        let check_priority = options.check_priority;
        let max_entries = options.max_entries;

        register_spsn_router!(
            SpsnNodeParenting,
            "SpsnNodeParenting",
            router_type,
            contact_plan,
            check_size,
            check_priority,
            max_entries
        );

        register_spsn_router!(
            SpsnNodeParentingHop,
            "SpsnNodeParentingHop",
            router_type,
            contact_plan,
            check_size,
            check_priority,
            max_entries
        );

        register_spsn_router!(
            SpsnHybridParenting,
            "SpsnHybridParenting",
            router_type,
            contact_plan,
            check_size,
            check_priority,
            max_entries
        );

        register_spsn_router!(
            SpsnHybridParentingHop,
            "SpsnHybridParentingHop",
            router_type,
            contact_plan,
            check_size,
            check_priority,
            max_entries
        );

        register_spsn_router!(
            SpsnContactParenting,
            "SpsnContactParenting",
            router_type,
            contact_plan,
            check_size,
            check_priority,
            max_entries
        );

        register_spsn_router!(
            SpsnContactParentingHop,
            "SpsnContactParentingHop",
            router_type,
            contact_plan,
            check_size,
            check_priority,
            max_entries
        );
    }

    register_cgr_router!(
        VolCgrNodeParenting,
        "VolCgrNodeParenting",
        router_type,
        contact_plan
    );

    register_cgr_router!(
        VolCgrHybridParenting,
        "VolCgrHybridParenting",
        router_type,
        contact_plan
    );

    register_cgr_router!(
        VolCgrHybridParentingHop,
        "VolCgrHybridParentingHop",
        router_type,
        contact_plan
    );

    register_cgr_router!(
        VolCgrNodeParentingHop,
        "VolCgrNodeParentingHop",
        router_type,
        contact_plan
    );

    register_cgr_router!(
        VolCgrContactParenting,
        "VolCgrContactParenting",
        router_type,
        contact_plan
    );

    register_cgr_router!(
        VolCgrContactParentingHop,
        "VolCgrContactParentingHop",
        router_type,
        contact_plan
    );

    #[cfg(feature = "contact_suppression")]
    register_cgr_router!(
        CgrFirstEndingHybridParentingHop,
        "CgrFirstEndingHybridParentingHop",
        router_type,
        contact_plan
    );

    #[cfg(feature = "contact_suppression")]
    register_cgr_router!(
        CgrFirstEndingHybridParenting,
        "CgrFirstEndingHybridParenting",
        router_type,
        contact_plan
    );

    #[cfg(feature = "contact_suppression")]
    register_cgr_router!(
        CgrFirstEndingNodeParentingHop,
        "CgrFirstEndingNodeParentingHop",
        router_type,
        contact_plan
    );

    #[cfg(feature = "contact_suppression")]
    register_cgr_router!(
        CgrFirstEndingNodeParenting,
        "CgrFirstEndingNodeParenting",
        router_type,
        contact_plan
    );

    #[cfg(feature = "contact_suppression")]
    register_cgr_router!(
        CgrFirstEndingContactParentingHop,
        "CgrFirstEndingContactParentingHop",
        router_type,
        contact_plan
    );

    #[cfg(feature = "contact_suppression")]
    register_cgr_router!(
        CgrFirstEndingContactParenting,
        "CgrFirstEndingContactParenting",
        router_type,
        contact_plan
    );

    #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
    register_cgr_router!(
        CgrFirstDepletedHybridParentingHop,
        "CgrFirstDepletedHybridParentingHop",
        router_type,
        contact_plan
    );

    #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
    register_cgr_router!(
        CgrFirstDepletedHybridParenting,
        "CgrFirstDepletedHybridParenting",
        router_type,
        contact_plan
    );

    #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
    register_cgr_router!(
        CgrFirstDepletedNodeParentingHop,
        "CgrFirstDepletedNodeParentingHop",
        router_type,
        contact_plan
    );

    #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
    register_cgr_router!(
        CgrFirstDepletedNodeParenting,
        "CgrFirstDepletedNodeParenting",
        router_type,
        contact_plan
    );

    #[cfg(all(
        feature = "contact_suppression",
        feature = "first_depleted"
    ))]
    register_cgr_router!(
        CgrFirstDepletedContactParentingHop,
        "CgrFirstDepletedContactParentingHop",
        router_type,
        contact_plan
    );

    #[cfg(all(
        feature = "contact_suppression",
        feature = "first_depleted"
    ))]
    register_cgr_router!(
        CgrFirstDepletedContactParenting,
        "CgrFirstDepletedContactParenting",
        router_type,
        contact_plan
    );

    Err(ASABRError::ScheduleError(
        "Router type is invalid! (check for typo, disabled feature, or missing options for Spsn algos)",
    ))
}
