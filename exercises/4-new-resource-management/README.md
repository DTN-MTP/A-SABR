# Creating new resource management techniques

## Motivation

As researchers, we want the ability to create new management techniques without the need to re-implement the other components that are unchanged, like the contact graph structure, the pathfinding approach, route selection and storage, and the routing mainframe.

As operators, mission-specific characteristics may justify the introduction of a new manager for specific contacts or nodes.



## New management techniques in A-SABR

As seen in the previous exercises, a compliant contact or node resource management technique is deployed as a structure that implements the `ContactManager` or `NodeManager` trait. In this example we will create a new manager that *could* render A-SABR compliant for routing in satellite constellation.

#### The manager

In a DTN, the bundle protocol allows the nodes to store the messages for arbitrarily long periods. We will assume that the main difference between DTN pathfinding and pathfinding in a constellation is the absence of message retention for the latter.

To this end, we can implement a node manager that disables the retention ability. This can also be done with a contact manager, we will see here how separation between link usage and node resource usage is processed.

This example is very simple and will assume that the sole constraint is the absence of storage capabilities. But of course, the manager can be extended to take other aspects into account, like buffer size, energy, etc.

The methods of the NodeManager traits are included in the control flow only if compilation features are enabled. For this task we want to control that the bundle is transmitted to the next node just after its arrival at the transmitting node, and we define a maximum treatment delay, for which a higher delay would be considered as retention:

```rust

#[cfg_attr(feature = "debug", derive(Debug))]
struct NoRetention {
    max_proc_time: Duration,
}

impl NodeManager for NoRetention {
    // See code for details
}
```


#### The parser
TODO: New interface

Now that the manager is ready, we can create elements of type `Node<NoRetention>` programmatically, but we are not yet ready to parse them from a contact plan. To do so, the `Parse<T>` trait must be implemented for `NoRetention`. This interface provides a `parse` class method that returns an element of type `T`, which we set to `NoRetention`. In other words, the library will be able to do `NoRetention::parse(...)` (internal machinery) to read the manager from the contact plan:


```rust
TODO: New interface
```


#### Dynamic parsing
TODO: New interface

