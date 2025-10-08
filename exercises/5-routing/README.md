# Routing


## Router structure

Composing a router can be a bit verbose. For example, the concrete type of the `VolCgr`:

``` rust
    VolCgr<NM, CM, NodeParentingPathExcl<NM, CM, SABR>, RoutingTable<NM, CM, SABR>>
```

`Hop` and `SABR ` are distance calculation strategies.

*Note: Although some templates are repeated, this does not include overhead thanks to the zero-sized template types. The compiler just requires an exact match between the types (`Path` for single destination variant, and `Excl` to include the node exclusion).*

When an alternative pathfinding algorithm is required, for example, `FirstDepleted`, the concrete type can slightly change:
``` rust
Cgr<NM, CM, FirstDepleted<NM, CM, HybridParentingPath<NM, CM, Hop>>, RoutingTable<NM, CM, Hop>>;
```

Indeed, the `FirstDepleted` approach depends on a pathfinding backend, so it is itself templated by `NodeParentingPath`.

With this templating/trait approach, the route storage strategy, the pathfinding strategy, and the distance calculation can be replaced.

## Aliasing

The concrete types are a bit verbose: to use the approaches already provided by the library, type aliasing is provided:

```rust
pub type VolCgrNodeParenting<NM, CM> =
    VolCgr<NM, CM, NodeParentingPathExcl<NM, CM, SABR>, RoutingTable<NM, CM, SABR>>;
pub type CgrFirstDepletedHybridParentingHop<NM, CM> =
    Cgr<NM, CM, FirstDepleted<NM, CM, HybridParentingPath<NM, CM, Hop>>, RoutingTable<NM, CM, Hop>>;
```

The alias is still templated by a `NodeManager` and a `ContactManager`, which is practical to limit the aliasing pressure on the names, and those templates appear for the contact plan parsing anyway. For the moment, SPSN and CGR (both `Cgr` & `VolCgr`) have a single route storage strategy.


## Generate a router

The library exposes the `build_generic_router` helper function to build a router by name. Here is an example for `VolCgrNodeParenting`:

```rust
    let mut router = build_generic_router::<NoManagement, EVLManager>(
        "VolCgrNodeParenting",
        nodes,
        contacts,
        None,
    );
```

To use the router, we need an abstraction of the bundle. `destinations` is currently a vector to support multicast, but it may become an enum in the future (Rust enums are very powerful).

```rust
    let bundle = Bundle {
        source: 0,
        destinations: vec![4],
        priority: 0,
        size: 10.0,
        expiration: 1000.0,
    };
```

Finally, the bundle can be scheduled. We omitted some layers, but the router here takes care of selection, etc.

```rust
    let out = router.route(0, &bundle, 0.0, &Vec::new());
```
And some helper functions are provided for smoother handling of the output.
```rust
    let (_first_hop_contact, route) = out.lazy_get_for_unicast(4).unwrap();
    pretty_print(route);
```