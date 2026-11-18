## Buffer-aware routing

### Run the example

```bash
cargo run --example buffer_routing
```

### Context

The `buffer_management` example calls `BufferManager` directly. This example shows the same node manager used by a router: when an intermediate node's buffer is full, the router chooses another path instead of routing through that node again.

### Scenario

Four nodes, loaded from `contact_plan.asabr` with `BufferManager` for nodes and `EVLManager` for contacts:

```
        a -> b   [0, 100]          b -> d   [50, 150]
   a <                        > d
        a -> c   [0, 100]          c -> d   [70, 170]
```

- b and c can each store **one** bundle (capacity 10, bundle size 10).
- The path through b reaches d earlier (b -> d opens at t=50, c -> d at t=70), so the router prefers b.

Two bundles of size 10 are routed from a to d at t=0, using `SpsnHybridParenting`.

### Behavior

**Bundle 1** goes a -> b -> d and arrives at t=60. Committing the route books b's buffer from the bundle's reception (t=0) until it leaves on b -> d (t=60).

**Bundle 2** would also arrive earliest through b, but b's buffer is full until t=60. `dry_run_retention` refuses it, so the router takes a -> c -> d and the bundle arrives at t=80.

```
b's buffer (as booked by the router)
volume
|
|  20 |    bundle 2 refused (20 > 10)
|  10 +----------------------------- capacity
|     |///////// bundle 1 /////////|
|   0 +----------------------------+---------> time
      t=0                         t=60
```

If b's capacity is raised to 20 in the contact plan, both bundles go through b.
