use crate::{
    distance::{hop::Hop, sabr::SABR},
    pathfinding::{
        contact_graph::{ContactGraphPath, ContactGraphTree},
        first_depleted::FirstDepleted,
        first_ending::FirstEnding,
        mpt::{MptPath, MptTree},
        node_graph::{NodeGraphPath, NodeGraphTree},
    },
    route_storage::{cache::TreeCache, table::RoutingTable},
};

use super::{cgr::Cgr, spsn::Spsn};

pub type SpsnMpt<NM, CM> = Spsn<NM, CM, MptTree<NM, CM, SABR>, TreeCache<NM, CM>>;
pub type SpsnNodeGraph<NM, CM> = Spsn<NM, CM, NodeGraphTree<NM, CM, SABR>, TreeCache<NM, CM>>;
pub type SpsnContactGraph<NM, CM> = Spsn<NM, CM, ContactGraphTree<NM, CM, SABR>, TreeCache<NM, CM>>;
pub type CgrFirstEndingMpt<NM, CM> =
    Cgr<NM, CM, FirstEnding<NM, CM, MptPath<NM, CM, SABR>>, RoutingTable<NM, CM, SABR>>;
pub type CgrFirstDepletedMpt<NM, CM> =
    Cgr<NM, CM, FirstDepleted<NM, CM, MptPath<NM, CM, SABR>>, RoutingTable<NM, CM, SABR>>;
pub type CgrFirstEndingNodeGraph<NM, CM> =
    Cgr<NM, CM, FirstEnding<NM, CM, NodeGraphPath<NM, CM, SABR>>, RoutingTable<NM, CM, SABR>>;
pub type CgrFirstDepletedNodeGraph<NM, CM> =
    Cgr<NM, CM, FirstDepleted<NM, CM, NodeGraphPath<NM, CM, SABR>>, RoutingTable<NM, CM, SABR>>;
pub type CgrFirstEndingContactGraph<NM, CM> =
    Cgr<NM, CM, FirstEnding<NM, CM, ContactGraphPath<NM, CM, SABR>>, RoutingTable<NM, CM, SABR>>;
pub type CgrFirstDepletedContactGraph<NM, CM> =
    Cgr<NM, CM, FirstDepleted<NM, CM, ContactGraphPath<NM, CM, SABR>>, RoutingTable<NM, CM, SABR>>;

pub type SpsnHopMpt<NM, CM> = Spsn<NM, CM, MptTree<NM, CM, Hop>, TreeCache<NM, CM>>;
pub type SpsnHopNodeGraph<NM, CM> = Spsn<NM, CM, NodeGraphTree<NM, CM, Hop>, TreeCache<NM, CM>>;
pub type SpsnHopContactGraph<NM, CM> =
    Spsn<NM, CM, ContactGraphTree<NM, CM, Hop>, TreeCache<NM, CM>>;
pub type CgrHopFirstEndingMpt<NM, CM> =
    Cgr<NM, CM, FirstEnding<NM, CM, MptPath<NM, CM, Hop>>, RoutingTable<NM, CM, Hop>>;
pub type CgrHopFirstDepletedMpt<NM, CM> =
    Cgr<NM, CM, FirstDepleted<NM, CM, MptPath<NM, CM, Hop>>, RoutingTable<NM, CM, Hop>>;
pub type CgrHopFirstEndingNodeGraph<NM, CM> =
    Cgr<NM, CM, FirstEnding<NM, CM, NodeGraphPath<NM, CM, Hop>>, RoutingTable<NM, CM, Hop>>;
pub type CgrHopFirstDepletedNodeGraph<NM, CM> =
    Cgr<NM, CM, FirstDepleted<NM, CM, NodeGraphPath<NM, CM, Hop>>, RoutingTable<NM, CM, Hop>>;
pub type CgrHopFirstEndingContactGraph<NM, CM> =
    Cgr<NM, CM, FirstEnding<NM, CM, ContactGraphPath<NM, CM, Hop>>, RoutingTable<NM, CM, Hop>>;
pub type CgrHopFirstDepletedContactGraph<NM, CM> =
    Cgr<NM, CM, FirstDepleted<NM, CM, ContactGraphPath<NM, CM, Hop>>, RoutingTable<NM, CM, Hop>>;
