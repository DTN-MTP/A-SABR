#![no_std]

//! A-SABR routing library.
//!
//! This crate provides contact-plan parsing, graph construction, pathfinding,
//! and routing helpers for scheduled bundle routing.

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
/// Module containing the vnode definition.
pub mod vnode;

/// Module containing the bundle definition.
pub mod bundle;

/// Module containing the data structure storing the nodes and contacts.
/// The structure does not influence the pathfinding implementations.
pub mod multigraph;
/// Module containing the different pathfinding implementations.
pub mod pathfinding;
/// Module containing the PathFragment definition.
pub mod paths;

///  Module containing the storage capabilities for the routes.
pub mod route_storage;

/// Module containing the logic to read a contact plan.
pub mod contact_plan;
/// Module containing the logic to enable different distance comparison strategy between two paths.
pub mod distance;
/// Module containing the logic to enable parsing abilities for the components.
pub mod parsing;

/// Utility helpers shared by the crate.
pub mod utils;

/// Module containing the polynomial modeling of contact capacity.
pub mod poly;

#[macro_use]
extern crate static_assertions;
