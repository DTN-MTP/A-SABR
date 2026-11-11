## Buffer Management

### Run the example

```bash
cargo run --example buffer_management
```

### Context

`BufferManager` tracks the available memory of a node over time. When routing bundles, it ensures that the node's storage capacity is never exceeded during the bundle's retention period (from reception to transmission).

In this example, the contact plan is parsed statically using `BufferManager` for the nodes and `ETOManager` for the contacts. In the A-SABR contact plan file, the initial volume capacity is provided directly after the node name:

```
node <id> <name> <capacity>
```

### Scenario

The script loads a contact plan from `buffer_format.asabr` where node 1 (Satellite) has a maximum memory capacity of 1000.

We simulate two bundles requiring storage on the Satellite during the exact same time window (reception at t=0, transmission between t=50 and t=100):

- A first bundle of size 600
- A second bundle of size 500

### Behavior

The `BufferManager` evaluates both retention requests using `dry_run_retention`.

The first bundle is accepted and its memory is committed since its size (600) is within the 1000 capacity limit. When the routing algorithm attempts to reserve space for the second bundle, the manager detects that the combined size (1100) exceeds the maximum capacity and safely rejects the second request.
