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

We simulate three bundles requiring storage on the Satellite with overlapping retention windows to demonstrate temporal tracking :
- **Bundle 1**: size 600, retained from t=0 to t=70.
- **Bundle 2**: size 300, retained from t=20 to t=90 (released after Bundle 1).
- **Bundle 3**: size 250, attempting to be retained from t=40 to t=60.

```
volume
|
| 1150 |             b3 (250) refused
|      |
| 1000 +--------------------------------------------- buffer capacity
|      |
|  900 |       +---------------+
|      |       |    b1 + b2    |
|  600 |   +---+               |
|      |   | b1 only           |
|      |   |                   |
|  300 |   |                   +-------+
|      |   |                   |b2 only|
|    0 +---+---+-------+-------+-------+---+--------> time
           |   |       |       |       |
          t=0 t=20    t=40    t=70    t=90
```

### Behavior

The `BufferManager` evaluates both retention requests using `dry_run_retention`.

**Bundle 1 (600)** is accepted and committed (600 <= 1000).

**Bundle 2 (300)** is accepted and committed, pushing peak volume to 900 strictly between t=20 and t=70.

**Bundle 3 (250)** is rejected. Even though the buffer would have enough space before t=20 or after t=70, accommodating bundle 3 strictly between t=40 and t=60 would push the total volume to 1150 (since both b1 and b2 are present in that window), safely preventing a capacity breach.
