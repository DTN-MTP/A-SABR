## Inter-regional routing and static anycast support

### Run the example

```bash
cargo run --example inter-regional_routing
```

### Motivation

Network authorities will administer nodes in separate autonomous systems, called here "regions", which may be managed by different actors. A-SABR proposes two regional interfacing methods.

The node passageway [1] approach consists of allowing nodes to be part of two regions simultaneously. Inter-regional routing to a neighboring region can be processed as a unicast transmission to the closest (in time) node passageway part of this neighboring region. A node passageway must therefore manage the contact plans of the two regions it bridges.

The contact passageway [2] approach allows complete separation, only the contacts that bridge the two regions are shared. Inter-regional routing to a neighboring region can be processed as a unicast transmission to the closest (in time) egress contact with this region.

### Prerequisites

This example only covers routing to neighbor regions (or anycast group), and A-SABR is not aware of EIDs. Only the home region's contact plan will be presented, and middleware must be deployed above A-SABR for possible EID<->NodeID translations.

The approach implements a static support for anycast, thanks to the assumption that regional membership will most likely be stable, allowing algorithmic optimization for "stable groups" described in the contact plan.

This approach can be leveraged for intra-regional static anycast support, but a dynamic approach (that does not require specific contact plan entries) can be preferable for other use cases.

### Scenario

#### Target topology

- The **B**lue region, administrated by **B**laise (local/home region)
- The **R**ed region, administrated by **R**ose
- The **G**reen region, administrated by **G**reg

![IRR contact plan](images/irr-cp.svg)

**B**laise organizes the bridging with the two neighboring regions **R** and **G**.

Sharing contact plans is acceptable for both **B**laise and **R**ose, and the bridging is organized with two node passageways, one owned by **B**laise (node 3) and one owned by **R**ose (node 4). Although not depicted, the node 3 of **B**laise sees all the contacts in region **R** (as does node 4), and node **4** of **R**ose needs to see all the contacts depicted in the figure.

However, **G**reg refuses to share internal information and agrees with **B**laise that only the contacts 2→7, 6→1, and 5→0 will be shared. Those contacts are referred to as contact passageways.

#### Implementation

A-SABR introduces two new contact plan elements to implement static anycast: "virtual nodes" and "external nodes".

![resulting graph](images/irr-graph.svg)

Rather than labeling the nodes with anycast membership (this can be a good option for dynamic anycast), A-SABR introduces pathfinding optimization with virtual node/vertex abstraction, aggregating the nodes sharing the same label. The relationship between virtual nodes and nodes is many-to-many.

**A `vnode` carries a valid ID for routing, anycast routing is abstracted by routing to the corresponding `vnode`.**

Declaring static anycast membership requires the `vnode` marker:

`vnode <id> <alias> [ <id> <id> .. ]`

The `enode` marker is also introduced for real nodes that are not members of the home region, called here external nodes. The IDs of the nodes abstracted by a vnode (the IDs between brackets) must be IDs of real nodes, either internal (`node`) or external (`enode`).

The resulting multigraph will present one extra vertex per vnode, in a vertex contraction manner. The original vertices of the nodes being merged are not removed after contraction. However, the contact reattachment policy depends on the nature of the nodes being merged:

- For a contact from or to a `node`, the contact will be *duplicated*, to get one *copy* attached to the `vnode`. One contact *copy* remains attached to the original node. Contact *copies* are, in fact, references to the same contact to keep resource awareness consistent.
- For a contact from or to an `enode`, the contact will be *reattached* to the vnode. As a result, the vertices associated with external nodes are *detached* in the graph.

The motivation for such handling is performance:

- With *duplication*, the original vertex of the `node` remains attached, routing to this node remains possible for intra-regional support.
- *Reattaching* the contacts with the `vnode` in order to *detach* (disconnect) the `enode` from the graph  **de-facto reduces its size**, by pruning the graph from vertices that are useless. Indeed, the EIDs associated with external nodes are most likely not shared with the home region. Supporting direct routing to a specific `enode` is allegedly irrelevant.

### References

[1] <https://amslaurea.unibo.it/id/eprint/17468/1/tesi_alessi.pdf>

[2] <https://hal.science/hal-04711330/file/_Juan_Olivier__Inter_Regional_Routing_Architecture.pdf>
