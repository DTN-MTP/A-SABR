## A* Routing

### Run the example

```bash
cargo run --example astar_routing
```

### Context

`AStar<D>` wraps a distance `D` (here `SABR`) and orders the candidate paths by their arrival time plus a heuristic: an estimate of the remaining delay to the destination. Nodes whose estimate shows they cannot lead to an early arrival are explored later, or not at all.

The heuristic is provided by the node managers. `AStar` requires the node manager type to implement the `NodeHeuristic` trait, so using it with managers that provide no heuristic (e.g. plain `NoManagement`) does not compile.

`DelayHeuristicManager<NM>` wraps any node manager `NM`: resource management is delegated to `NM`, and the heuristic comes from a row of delays, one per destination node. In an A-SABR contact plan, the row comes right after the node name, followed by the tokens of `NM` (none for `NoManagement`):

```
node <id> <name> [<delay to node 0>, <delay to node 1>, ...]
```

### Scenario

The network has four nodes. Node 0 is the source and node 3 the destination. There are two paths:

- through node 1: propagation delay of 10 on each contact, both contacts always available. The bundle arrives at t=20.
- through node 2: propagation delay of 1 on each contact, but node 2 can only reach node 3 from t=50. The bundle arrives at t=51.

In `contact_plan.cp`, each row entry is the smallest total propagation delay between the two nodes. These are lower bounds on the real delay, since waiting for contacts is ignored.

### Behavior

With the lower bounds, `AStar<SABR>` returns the same route as `SABR`: through node 1, arriving at t=20.
