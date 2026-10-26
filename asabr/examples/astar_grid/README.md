## A* on a Grid

### Run the example

```bash
cargo run --example astar_grid
```

### Context

`AStar<SABR>` returns the same route as `SABR` as long as the heuristic never overestimates the remaining delay. What changes is how much of the graph is explored: the pathfinder stops as soon as the destination is reached, and the heuristic steers it towards the destination instead of expanding in every direction.

### Scenario

The network is a 30x30 grid. Each node is linked to its 4 neighbours by contacts that are always available, with a delay of 1. The bundle goes from the middle of the left edge to the middle of the right edge.

The contact plan is generated in code, in the A-SABR format. Each node carries a `DelayHeuristicManager` whose row is its Manhattan distance to every other node, multiplied by the hop delay. On a grid, this is a lower bound on the delay.

To measure the work done, the heuristic wraps a small `Counting` node manager, defined in the example, that counts the calls to `accept`: one per hop the pathfinder tries. This also shows the nesting of node managers: `DelayHeuristicManager<Counting>` provides the heuristic, and delegates resource management to `Counting`.

It also measures the time it takes to find a path, averaged over 200 runs.

### Behavior

Both distances find the same route (29 hops, arrival at t=29). `SABR` explores most of the grid before reaching the destination, while `AStar<SABR>` only follows the nodes leading to it:

```
30x30 grid, from node 450 to node 479

distance      hops tried  time/search    result
SABR                1324       44.7µs    Route to enode: 479 at t=29 with 29 hop(s):
AStar<SABR>           87        4.9µs    Route to enode: 479 at t=29 with 29 hop(s):
```

Run ```cargo run --release --example astar_grid``` to get more meaningful timing numbers. Note that the time/search depends of the machine but hops tried doesn't.
