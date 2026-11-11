## Temporal Buffer Management

### Run the example

```bash
cargo run --example buffer_management
```

### Context

In typical DTN, nodes (such as satellites or rovers) suffer from strict storage constraints. Standard routing algorithms need to ensure that a node's memory is never overflowed at any given time $t$ before committing a bundle to a specific path.

This feature introduces a highly efficient BufferManager. Instead of relying on a linear Vec representation of past events, the manager utilizes a BTreeMap to store volume transitions as a time-series.

### Scenario

The network encompasses three nodes: Earth (0), a relay Satellite (1), and Mars (2).

The nodes are statically parsed using the BufferManager, where the integer following the node name directly initializes its capacity.

The Satellite has a strictly limited capacity of 1000 units.

### Behavior

- Successful Allocation: When a bundle of size 600 is evaluated, the BufferManager queries the active volume at the exact start time using an $\mathcal{O}(\log N)$ backward iterator (next_back()). It verifies that no future transitions within the retention window exceed the capacity, and successfully commits the memory allocation.

- Safe Rejection: When a second bundle of size 500 attempts to use the exact same time window, the manager instantly detects that the sum ($600 + 500 = 1100$) exceeds the capacity constraint of 1000, safely rejecting the operation.

### Implementation concerns

By using `alloc::collections::BTreeMap`, lookups and insertions drop from $\mathcal{O}(N)$ to $\mathcal{O}(\log N)$ complexity. During a commit, the manager explicitly mutates only the relevant time window using range_mut(), entirely avoiding the massive cache-misses and memory shifting issues associated with vector-based timelines.

The engine is strictly no_std compliant.
