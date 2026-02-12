/// Module containing the adaptive contact definition.
pub mod contact;
/// Module containing the variable component of a contact for resource management.
pub mod contact_manager;
/// Module containing the library error types.
pub mod errors;
/// Module containing the adaptive node definition.
pub mod node;
/// Module containing the variable component of a node for resource management.
pub mod node_manager;
/// Module containing the library primitive types.
pub mod types;

/// Module containing the bundle definition.
pub mod bundle;

/// Module containing the data structure storing the nodes and contacts.
/// The structure does not influence the pathfinding implementations.
pub mod multigraph;
/// Module containing the different pathfinding implementations.
pub mod pathfinding;
/// Module containing the RouteStage definition.
/// A RouteStage is an abstraction of Dijkstra's algorithm progress, a route hop, or work areas.
pub mod route_stage;

///  Module containing the storage capabilities for the routes.
pub mod route_storage;
///  Module containing the routing algorithms.
pub mod routing;

/// Module containing the logic to read a contact plan.
pub mod contact_plan;
/// Module containing the logic to enable different distance comparison strategy between two paths.
pub mod distance;
/// Module containing the logic to enable parsing abilities for the components.
pub mod parsing;

pub mod utils;
