## Inter-regional routing

### Run the example

This example does not require any features yet, but we may add a `vnode_routing` (or similar) feature in the future:

```bash
cargo run --example inter-regional_routing
```

### Motivation

Network authorities administer nodes in separate 'regions' (this term lacks a reference). This leads to concerns about inter-regional routing: one authority may want to allows other authorities to cross its regions without exposing extensive informations internal to the regions, such as nodes, node managers, contacts, etc.
This exposes the need to abstract regions as 'black boxes', as well as the need for anycast routing, to the closest contact to a given region.
Multiple approaches to the problem of inter-regional routing have surfaced (BIBE, node passageways, contact passageways). We focus on node passageways and contact passageways.

We wish to implement an abstraction over both.
In terms of research, this could lead to results such as finding one approach to be a more general case, or inversely a degenerate case of the other.
In turn we may find a common model in which both cases are degenerate cases of the model.

We also use this abstraction to compare both approaches on various criteria.

### Principles

A-SABR introduces contact plan elements to label nodes, in order to create "groups" or "virtual nodes" (`vnode` in the contact plan) for anycast routing; as well as "external nodes" (`enode` in the contact plan) which can only be routed to using anycast, and thus must be labelled with at least one vnode.
These elements are only available in the A-SABR contact plan format.

A node (`node` in the contact plan) is called an "internal node" or "inode" to differentiate it from an enode.

Contacts still only point to real nodes (i.e. inodes or enodes), they cannot point to vnodes as they are only labels, not real nodes.

During multigraph creation, the Sender/Receiver abstraction over the graph is extended with new vertices for each virtual node, whose set of contacts is the union of the contacts of the real nodes that the vnode labels.

External nodes enable efficient representation of nodes that hold few information or little interest for unicast routing, e.g. nodes of a separate region, as they are not added as Senders or Receivers in the graph. They only exist as contact Tx/Rx nodes.
External nodes are expected to most often have a `NoManager` manager.
