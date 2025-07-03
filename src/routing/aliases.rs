use crate::{
    contact::Contact,
    contact_manager::ContactManager,
    distance::{hop::Hop, sabr::SABR},
    node::Node,
    node_manager::NodeManager,
    pathfinding::{
        mpt::{MptPathExcl, MptTreeExcl},
        node_graph::{NodeGraphPathExcl, NodeGraphTreeExcl},
    },
    route_storage::{cache::TreeCache, table::RoutingTable},
    routing::volcgr::VolCgr,
};
use std::{cell::RefCell, rc::Rc};

#[cfg(feature = "contact_suppression")]
use super::cgr::Cgr;
#[cfg(all(feature = "contact_work_area", feature = "contact_suppression"))]
use crate::pathfinding::contact_graph::ContactGraphPath;
#[cfg(feature = "contact_work_area")]
use crate::pathfinding::contact_graph::{ContactGraphPathExcl, ContactGraphTreeExcl};

#[cfg(feature = "first_depleted")]
use crate::pathfinding::limiting_contact::first_depleted::FirstDepleted;
#[cfg(feature = "contact_suppression")]
use crate::pathfinding::limiting_contact::first_ending::FirstEnding;
#[cfg(feature = "contact_suppression")]
use crate::pathfinding::mpt::MptPath;
#[cfg(feature = "contact_suppression")]
use crate::pathfinding::node_graph::NodeGraphPath;

use super::{spsn::Spsn, Router};

pub type SpsnMpt<NM, CM> = Spsn<NM, CM, MptTreeExcl<NM, CM, SABR>, TreeCache<NM, CM>>;

pub type SpsnNodeGraph<NM, CM> = Spsn<NM, CM, NodeGraphTreeExcl<NM, CM, SABR>, TreeCache<NM, CM>>;

#[cfg(feature = "contact_work_area")]
pub type SpsnContactGraph<NM, CM> =
    Spsn<NM, CM, ContactGraphTreeExcl<NM, CM, SABR>, TreeCache<NM, CM>>;

pub type VolCgrMpt<NM, CM> = VolCgr<NM, CM, MptPathExcl<NM, CM, SABR>, RoutingTable<NM, CM, SABR>>;

pub type VolCgrNodeGraph<NM, CM> =
    VolCgr<NM, CM, NodeGraphPathExcl<NM, CM, SABR>, RoutingTable<NM, CM, SABR>>;

#[cfg(feature = "contact_work_area")]
pub type VolCgrContactGraph<NM, CM> =
    VolCgr<NM, CM, ContactGraphPathExcl<NM, CM, SABR>, RoutingTable<NM, CM, SABR>>;

#[cfg(feature = "contact_suppression")]
pub type CgrFirstEndingMpt<NM, CM> =
    Cgr<NM, CM, FirstEnding<NM, CM, MptPath<NM, CM, SABR>>, RoutingTable<NM, CM, SABR>>;

#[cfg(feature = "first_depleted")]
pub type CgrFirstDepletedMpt<NM, CM> =
    Cgr<NM, CM, FirstDepleted<NM, CM, MptPath<NM, CM, SABR>>, RoutingTable<NM, CM, SABR>>;

#[cfg(feature = "contact_suppression")]
pub type CgrFirstEndingNodeGraph<NM, CM> =
    Cgr<NM, CM, FirstEnding<NM, CM, NodeGraphPath<NM, CM, SABR>>, RoutingTable<NM, CM, SABR>>;

#[cfg(feature = "first_depleted")]
pub type CgrFirstDepletedNodeGraph<NM, CM> =
    Cgr<NM, CM, FirstDepleted<NM, CM, NodeGraphPath<NM, CM, SABR>>, RoutingTable<NM, CM, SABR>>;

#[cfg(all(feature = "contact_work_area", feature = "contact_suppression"))]
pub type CgrFirstEndingContactGraph<NM, CM> =
    Cgr<NM, CM, FirstEnding<NM, CM, ContactGraphPath<NM, CM, SABR>>, RoutingTable<NM, CM, SABR>>;

#[cfg(all(
    feature = "contact_work_area",
    feature = "contact_suppression",
    feature = "first_depleted"
))]
pub type CgrFirstDepletedContactGraph<NM, CM> =
    Cgr<NM, CM, FirstDepleted<NM, CM, ContactGraphPath<NM, CM, SABR>>, RoutingTable<NM, CM, SABR>>;

pub type SpsnHopMpt<NM, CM> = Spsn<NM, CM, MptTreeExcl<NM, CM, Hop>, TreeCache<NM, CM>>;

pub type SpsnHopNodeGraph<NM, CM> = Spsn<NM, CM, NodeGraphTreeExcl<NM, CM, Hop>, TreeCache<NM, CM>>;

#[cfg(feature = "contact_work_area")]
pub type SpsnHopContactGraph<NM, CM> =
    Spsn<NM, CM, ContactGraphTreeExcl<NM, CM, Hop>, TreeCache<NM, CM>>;

pub type VolCgrHopMpt<NM, CM> = VolCgr<NM, CM, MptPathExcl<NM, CM, Hop>, RoutingTable<NM, CM, Hop>>;

pub type VolCgrHopNodeGraph<NM, CM> =
    VolCgr<NM, CM, NodeGraphPathExcl<NM, CM, Hop>, RoutingTable<NM, CM, Hop>>;

#[cfg(feature = "contact_work_area")]
pub type VolCgrHopContactGraph<NM, CM> =
    VolCgr<NM, CM, ContactGraphPathExcl<NM, CM, Hop>, RoutingTable<NM, CM, Hop>>;

#[cfg(feature = "contact_suppression")]
pub type CgrHopFirstEndingMpt<NM, CM> =
    Cgr<NM, CM, FirstEnding<NM, CM, MptPath<NM, CM, Hop>>, RoutingTable<NM, CM, Hop>>;

#[cfg(feature = "first_depleted")]
pub type CgrHopFirstDepletedMpt<NM, CM> =
    Cgr<NM, CM, FirstDepleted<NM, CM, MptPath<NM, CM, Hop>>, RoutingTable<NM, CM, Hop>>;

#[cfg(feature = "contact_suppression")]
pub type CgrHopFirstEndingNodeGraph<NM, CM> =
    Cgr<NM, CM, FirstEnding<NM, CM, NodeGraphPath<NM, CM, Hop>>, RoutingTable<NM, CM, Hop>>;

#[cfg(feature = "first_depleted")]
pub type CgrHopFirstDepletedNodeGraph<NM, CM> =
    Cgr<NM, CM, FirstDepleted<NM, CM, NodeGraphPath<NM, CM, Hop>>, RoutingTable<NM, CM, Hop>>;

#[cfg(all(feature = "contact_work_area", feature = "contact_suppression"))]
pub type CgrHopFirstEndingContactGraph<NM, CM> =
    Cgr<NM, CM, FirstEnding<NM, CM, ContactGraphPath<NM, CM, Hop>>, RoutingTable<NM, CM, Hop>>;

#[cfg(all(
    feature = "contact_work_area",
    feature = "contact_suppression",
    feature = "first_depleted"
))]
pub type CgrHopFirstDepletedContactGraph<NM, CM> =
    Cgr<NM, CM, FirstDepleted<NM, CM, ContactGraphPath<NM, CM, Hop>>, RoutingTable<NM, CM, Hop>>;

macro_rules! register_cgr_router {
    ($router:ident, $router_name:literal, $test_name_variable:ident, $nodes:ident, $contacts:ident) => {
        if $test_name_variable == $router_name {
            let routing_table = Rc::new(RefCell::new(RoutingTable::new()));

            return Box::new($router::<NM, CM>::new($nodes, $contacts, routing_table));
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

            return Box::new($router::<NM, CM>::new(
                $nodes,
                $contacts,
                cache,
                $check_priority,
            ));
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
) -> Box<dyn Router<NM, CM>> {
    if let Some(options) = spsn_options {
        let check_size = options.check_size;
        let check_priority = options.check_priority;
        let max_entries = options.max_entries;

        register_spsn_router!(
            SpsnNodeGraph,
            "SpsnNodeGraph",
            router_type,
            nodes,
            contacts,
            check_size,
            check_priority,
            max_entries
        );
        register_spsn_router!(
            SpsnHopNodeGraph,
            "SpsnHopNodeGraph",
            router_type,
            nodes,
            contacts,
            check_size,
            check_priority,
            max_entries
        );
        register_spsn_router!(
            SpsnMpt,
            "SpsnMpt",
            router_type,
            nodes,
            contacts,
            check_size,
            check_priority,
            max_entries
        );
        register_spsn_router!(
            SpsnHopMpt,
            "SpsnHopMpt",
            router_type,
            nodes,
            contacts,
            check_size,
            check_priority,
            max_entries
        );
        register_cgr_router!(
            VolCgrNodeGraph,
            "VolCgrNodeGraph",
            router_type,
            nodes,
            contacts
        );
        register_cgr_router!(
            VolCgrHopNodeGraph,
            "VolCgrHopNodeGraph",
            router_type,
            nodes,
            contacts
        );
        register_cgr_router!(VolCgrMpt, "VolCgrMpt", router_type, nodes, contacts);
        register_cgr_router!(VolCgrHopMpt, "VolCgrHopMpt", router_type, nodes, contacts);

        #[cfg(feature = "contact_work_area")]
        {
            register_spsn_router!(
                SpsnContactGraph,
                "SpsnContactGraph",
                router_type,
                nodes,
                contacts,
                check_size,
                check_priority,
                max_entries
            );
            register_spsn_router!(
                SpsnHopContactGraph,
                "SpsnHopContactGraph",
                router_type,
                nodes,
                contacts,
                check_size,
                check_priority,
                max_entries
            );
            register_cgr_router!(
                VolCgrContactGraph,
                "VolCgrContactGraph",
                router_type,
                nodes,
                contacts
            );
            register_cgr_router!(
                VolCgrHopContactGraph,
                "VolCgrHopContactGraph",
                router_type,
                nodes,
                contacts
            );
        }
    }

    #[cfg(feature = "contact_suppression")]
    {
        register_cgr_router!(
            CgrHopFirstEndingMpt,
            "CgrHopFirstEndingMpt",
            router_type,
            nodes,
            contacts
        );
        register_cgr_router!(
            CgrFirstEndingMpt,
            "CgrFirstEndingMpt",
            router_type,
            nodes,
            contacts
        );
        register_cgr_router!(
            CgrHopFirstEndingNodeGraph,
            "CgrHopFirstEndingNodeGraph",
            router_type,
            nodes,
            contacts
        );
        register_cgr_router!(
            CgrFirstEndingNodeGraph,
            "CgrFirstEndingNodeGraph",
            router_type,
            nodes,
            contacts
        );

        #[cfg(feature = "contact_work_area")]
        {
            register_cgr_router!(
                CgrHopFirstEndingContactGraph,
                "CgrHopFirstEndingContactGraph",
                router_type,
                nodes,
                contacts
            );
            register_cgr_router!(
                CgrFirstEndingContactGraph,
                "CgrFirstEndingContactGraph",
                router_type,
                nodes,
                contacts
            );
        }
    }

    #[cfg(feature = "first_depleted")]
    {
        register_cgr_router!(
            CgrHopFirstDepletedMpt,
            "CgrHopFirstDepletedMpt",
            router_type,
            nodes,
            contacts
        );
        register_cgr_router!(
            CgrFirstDepletedMpt,
            "CgrFirstDepletedMpt",
            router_type,
            nodes,
            contacts
        );
        register_cgr_router!(
            CgrHopFirstDepletedNodeGraph,
            "CgrHopFirstDepletedNodeGraph",
            router_type,
            nodes,
            contacts
        );
        register_cgr_router!(
            CgrFirstDepletedNodeGraph,
            "CgrFirstDepletedNodeGraph",
            router_type,
            nodes,
            contacts
        );

        #[cfg(feature = "contact_work_area")]
        {
            register_cgr_router!(
                CgrHopFirstDepletedContactGraph,
                "CgrHopFirstDepletedContactGraph",
                router_type,
                nodes,
                contacts
            );
            register_cgr_router!(
                CgrFirstDepletedContactGraph,
                "CgrFirstDepletedContactGraph",
                router_type,
                nodes,
                contacts
            );
        }
    }

    panic!(
        "Router type \"{}\" is invalid! (check for typo, disabled feature, or missing options for Spsn algos)",
        &router_type
    );
}
