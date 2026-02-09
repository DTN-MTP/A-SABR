use crate::{
    contact::Contact,
    contact_manager::ContactManager,
    distance::{hop::Hop, sabr::SABR},
    errors::ASABRError,
    node::Node,
    node_manager::NodeManager,
    pathfinding::{
        hybrid_parenting::{HybridParentingPathExcl, HybridParentingTreeExcl},
        node_parenting::{NodeParentingPathExcl, NodeParentingTreeExcl},
    },
    route_storage::{cache::TreeCache, table::RoutingTable},
    routing::volcgr::VolCgr,
};
use std::{cell::RefCell, rc::Rc};

#[cfg(feature = "contact_suppression")]
use super::cgr::Cgr;
#[cfg(all(feature = "contact_work_area", feature = "contact_suppression"))]
use crate::pathfinding::contact_parenting::ContactParentingPath;
#[cfg(feature = "contact_work_area")]
use crate::pathfinding::contact_parenting::{ContactParentingPathExcl, ContactParentingTreeExcl};

#[cfg(feature = "contact_suppression")]
use crate::pathfinding::hybrid_parenting::HybridParentingPath;
#[cfg(feature = "first_depleted")]
use crate::pathfinding::limiting_contact::first_depleted::FirstDepleted;
#[cfg(feature = "contact_suppression")]
use crate::pathfinding::limiting_contact::first_ending::FirstEnding;
#[cfg(feature = "contact_suppression")]
use crate::pathfinding::node_parenting::NodeParentingPath;

use super::{spsn::Spsn, Router};

pub type SpsnHybridParenting<NM, CM> =
    Spsn<NM, CM, HybridParentingTreeExcl<NM, CM, SABR>, TreeCache<NM, CM>>;

pub type SpsnNodeParenting<NM, CM> =
    Spsn<NM, CM, NodeParentingTreeExcl<NM, CM, SABR>, TreeCache<NM, CM>>;

#[cfg(feature = "contact_work_area")]
pub type SpsnContactParenting<NM, CM> =
    Spsn<NM, CM, ContactParentingTreeExcl<NM, CM, SABR>, TreeCache<NM, CM>>;

pub type VolCgrHybridParenting<NM, CM> =
    VolCgr<NM, CM, HybridParentingPathExcl<NM, CM, SABR>, RoutingTable<NM, CM, SABR>>;

pub type VolCgrNodeParenting<NM, CM> =
    VolCgr<NM, CM, NodeParentingPathExcl<NM, CM, SABR>, RoutingTable<NM, CM, SABR>>;

#[cfg(feature = "contact_work_area")]
pub type VolCgrContactParenting<NM, CM> =
    VolCgr<NM, CM, ContactParentingPathExcl<NM, CM, SABR>, RoutingTable<NM, CM, SABR>>;

#[cfg(feature = "contact_suppression")]
pub type CgrFirstEndingHybridParenting<NM, CM> =
    Cgr<NM, CM, FirstEnding<NM, CM, HybridParentingPath<NM, CM, SABR>>, RoutingTable<NM, CM, SABR>>;

#[cfg(feature = "first_depleted")]
pub type CgrFirstDepletedHybridParenting<NM, CM> = Cgr<
    NM,
    CM,
    FirstDepleted<NM, CM, HybridParentingPath<NM, CM, SABR>>,
    RoutingTable<NM, CM, SABR>,
>;

#[cfg(feature = "contact_suppression")]
pub type CgrFirstEndingNodeParenting<NM, CM> =
    Cgr<NM, CM, FirstEnding<NM, CM, NodeParentingPath<NM, CM, SABR>>, RoutingTable<NM, CM, SABR>>;

#[cfg(feature = "first_depleted")]
pub type CgrFirstDepletedNodeParenting<NM, CM> =
    Cgr<NM, CM, FirstDepleted<NM, CM, NodeParentingPath<NM, CM, SABR>>, RoutingTable<NM, CM, SABR>>;

#[cfg(all(feature = "contact_work_area", feature = "contact_suppression"))]
pub type CgrFirstEndingContactParenting<NM, CM> = Cgr<
    NM,
    CM,
    FirstEnding<NM, CM, ContactParentingPath<NM, CM, SABR>>,
    RoutingTable<NM, CM, SABR>,
>;

#[cfg(all(feature = "contact_work_area", feature = "first_depleted"))]
pub type CgrFirstDepletedContactParenting<NM, CM> = Cgr<
    NM,
    CM,
    FirstDepleted<NM, CM, ContactParentingPath<NM, CM, SABR>>,
    RoutingTable<NM, CM, SABR>,
>;

pub type SpsnHybridParentingHop<NM, CM> =
    Spsn<NM, CM, HybridParentingTreeExcl<NM, CM, Hop>, TreeCache<NM, CM>>;

pub type SpsnNodeParentingHop<NM, CM> =
    Spsn<NM, CM, NodeParentingTreeExcl<NM, CM, Hop>, TreeCache<NM, CM>>;

#[cfg(feature = "contact_work_area")]
pub type SpsnContactParentingHop<NM, CM> =
    Spsn<NM, CM, ContactParentingTreeExcl<NM, CM, Hop>, TreeCache<NM, CM>>;

pub type VolCgrHybridParentingHop<NM, CM> =
    VolCgr<NM, CM, HybridParentingPathExcl<NM, CM, Hop>, RoutingTable<NM, CM, Hop>>;

pub type VolCgrNodeParentingHop<NM, CM> =
    VolCgr<NM, CM, NodeParentingPathExcl<NM, CM, Hop>, RoutingTable<NM, CM, Hop>>;

#[cfg(feature = "contact_work_area")]
pub type VolCgrContactParentingHop<NM, CM> =
    VolCgr<NM, CM, ContactParentingPathExcl<NM, CM, Hop>, RoutingTable<NM, CM, Hop>>;

#[cfg(feature = "contact_suppression")]
pub type CgrFirstEndingHybridParentingHop<NM, CM> =
    Cgr<NM, CM, FirstEnding<NM, CM, HybridParentingPath<NM, CM, Hop>>, RoutingTable<NM, CM, Hop>>;

#[cfg(feature = "first_depleted")]
pub type CgrFirstDepletedHybridParentingHop<NM, CM> =
    Cgr<NM, CM, FirstDepleted<NM, CM, HybridParentingPath<NM, CM, Hop>>, RoutingTable<NM, CM, Hop>>;

#[cfg(feature = "contact_suppression")]
pub type CgrFirstEndingNodeParentingHop<NM, CM> =
    Cgr<NM, CM, FirstEnding<NM, CM, NodeParentingPath<NM, CM, Hop>>, RoutingTable<NM, CM, Hop>>;

#[cfg(feature = "first_depleted")]
pub type CgrFirstDepletedNodeParentingHop<NM, CM> =
    Cgr<NM, CM, FirstDepleted<NM, CM, NodeParentingPath<NM, CM, Hop>>, RoutingTable<NM, CM, Hop>>;

#[cfg(all(feature = "contact_work_area", feature = "contact_suppression"))]
pub type CgrFirstEndingContactParentingHop<NM, CM> =
    Cgr<NM, CM, FirstEnding<NM, CM, ContactParentingPath<NM, CM, Hop>>, RoutingTable<NM, CM, Hop>>;

#[cfg(all(feature = "contact_work_area", feature = "first_depleted"))]
pub type CgrFirstDepletedContactParentingHop<NM, CM> = Cgr<
    NM,
    CM,
    FirstDepleted<NM, CM, ContactParentingPath<NM, CM, Hop>>,
    RoutingTable<NM, CM, Hop>,
>;

macro_rules! register_cgr_router {
    ($router:ident, $router_name:literal, $test_name_variable:ident, $nodes:ident, $contacts:ident) => {
        if $test_name_variable == $router_name {
            let routing_table = Rc::new(RefCell::new(RoutingTable::new()));

            return Ok(Box::new($router::<NM, CM>::new(
                $nodes,
                $contacts,
                routing_table,
            )));
        }
    };
}

macro_rules! register_spsn_router {
    ($router:ident, $router_name:literal, $test_name_variable:ident, $nodes:ident, $contacts:ident, $check_size:ident, $check_priority:ident, $max_entries:ident) => {
        if $test_name_variable == $router_name {
            let cache = Rc::new(RefCell::new(TreeCache::new(
                $check_size,
                $check_priority,
                $max_entries,
            )));

            return Ok(Box::new($router::<NM, CM>::new(
                $nodes,
                $contacts,
                cache,
                $check_priority,
            )));
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
    nodes: Vec<Node<NM>>,
    contacts: Vec<Contact<NM, CM>>,
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
            nodes,
            contacts,
            check_size,
            check_priority,
            max_entries
        );

        register_spsn_router!(
            SpsnNodeParentingHop,
            "SpsnNodeParentingHop",
            router_type,
            nodes,
            contacts,
            check_size,
            check_priority,
            max_entries
        );

        register_spsn_router!(
            SpsnHybridParenting,
            "SpsnHybridParenting",
            router_type,
            nodes,
            contacts,
            check_size,
            check_priority,
            max_entries
        );

        register_spsn_router!(
            SpsnHybridParentingHop,
            "SpsnHybridParentingHop",
            router_type,
            nodes,
            contacts,
            check_size,
            check_priority,
            max_entries
        );

        #[cfg(feature = "contact_work_area")]
        register_spsn_router!(
            SpsnContactParenting,
            "SpsnContactParenting",
            router_type,
            nodes,
            contacts,
            check_size,
            check_priority,
            max_entries
        );

        #[cfg(feature = "contact_work_area")]
        register_spsn_router!(
            SpsnContactParentingHop,
            "SpsnContactParentingHop",
            router_type,
            nodes,
            contacts,
            check_size,
            check_priority,
            max_entries
        );
    }

    register_cgr_router!(
        VolCgrNodeParenting,
        "VolCgrNodeParenting",
        router_type,
        nodes,
        contacts
    );

    register_cgr_router!(
        VolCgrHybridParenting,
        "VolCgrHybridParenting",
        router_type,
        nodes,
        contacts
    );

    register_cgr_router!(
        VolCgrHybridParentingHop,
        "VolCgrHybridParentingHop",
        router_type,
        nodes,
        contacts
    );

    register_cgr_router!(
        VolCgrNodeParentingHop,
        "VolCgrNodeParentingHop",
        router_type,
        nodes,
        contacts
    );

    #[cfg(feature = "contact_work_area")]
    register_cgr_router!(
        VolCgrContactParenting,
        "VolCgrContactParenting",
        router_type,
        nodes,
        contacts
    );

    #[cfg(feature = "contact_work_area")]
    register_cgr_router!(
        VolCgrContactParentingHop,
        "VolCgrContactParentingHop",
        router_type,
        nodes,
        contacts
    );

    #[cfg(feature = "contact_suppression")]
    register_cgr_router!(
        CgrFirstEndingHybridParentingHop,
        "CgrFirstEndingHybridParentingHop",
        router_type,
        nodes,
        contacts
    );

    #[cfg(feature = "contact_suppression")]
    register_cgr_router!(
        CgrFirstEndingHybridParenting,
        "CgrFirstEndingHybridParenting",
        router_type,
        nodes,
        contacts
    );

    #[cfg(feature = "contact_suppression")]
    register_cgr_router!(
        CgrFirstEndingNodeParentingHop,
        "CgrFirstEndingNodeParentingHop",
        router_type,
        nodes,
        contacts
    );

    #[cfg(feature = "contact_suppression")]
    register_cgr_router!(
        CgrFirstEndingNodeParenting,
        "CgrFirstEndingNodeParenting",
        router_type,
        nodes,
        contacts
    );

    #[cfg(all(feature = "contact_work_area", feature = "contact_suppression"))]
    register_cgr_router!(
        CgrFirstEndingContactParentingHop,
        "CgrFirstEndingContactParentingHop",
        router_type,
        nodes,
        contacts
    );

    #[cfg(all(feature = "contact_work_area", feature = "contact_suppression"))]
    register_cgr_router!(
        CgrFirstEndingContactParenting,
        "CgrFirstEndingContactParenting",
        router_type,
        nodes,
        contacts
    );

    #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
    register_cgr_router!(
        CgrFirstDepletedHybridParentingHop,
        "CgrFirstDepletedHybridParentingHop",
        router_type,
        nodes,
        contacts
    );

    #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
    register_cgr_router!(
        CgrFirstDepletedHybridParenting,
        "CgrFirstDepletedHybridParenting",
        router_type,
        nodes,
        contacts
    );

    #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
    register_cgr_router!(
        CgrFirstDepletedNodeParentingHop,
        "CgrFirstDepletedNodeParentingHop",
        router_type,
        nodes,
        contacts
    );

    #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
    register_cgr_router!(
        CgrFirstDepletedNodeParenting,
        "CgrFirstDepletedNodeParenting",
        router_type,
        nodes,
        contacts
    );

    #[cfg(all(
        feature = "contact_work_area",
        feature = "contact_suppression",
        feature = "first_depleted"
    ))]
    register_cgr_router!(
        CgrFirstDepletedContactParentingHop,
        "CgrFirstDepletedContactParentingHop",
        router_type,
        nodes,
        contacts
    );

    #[cfg(all(
        feature = "contact_work_area",
        feature = "contact_suppression",
        feature = "first_depleted"
    ))]
    register_cgr_router!(
        CgrFirstDepletedContactParenting,
        "CgrFirstDepletedContactParenting",
        router_type,
        nodes,
        contacts
    );

    Err(ASABRError::ScheduleError("Router type is invalid! (check for typo, disabled feature, or missing options for Spsn algos)"))
}
