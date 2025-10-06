# Creating new resource management techniques

## Motivation

As researchers, we want the ability to create new management techniques without the need to re-implement the other components that are unchanged, like the contact graph structure, the pathfinding approach, route selection and storage, and the routing mainframe.

As operators, mission-specific characteristics may justify the introduction of a new manager for specific contacts or nodes.



## New management techniques in A-SABR

As seen in the previous exercises, a compliant contact or node resource management technique is deployed as a structure that implements the `ContactManager` of `NodeManager` trait. In this example we will create a new manager that *could* render A-SABR compliant for routing in satellite constellation.

#### The manager

In a DTN, the bundle protocol allows the nodes to store the messages for arbitrarily long periods. We will assume that the main difference between DTN pathfinding and pathfinding in a constellation is the absence of message retention for the latter.

To this end, we can implement a node manager that disables the retention ability. This can also be done with a contact manager, we will see here how separation between link usage and node resource usage is processed.

This example is very simple and will assume that the sole constraint is the absence of storage capabilities. But of course, the manager can be extended to take other aspects into account, like buffer size, energy, etc.

The methods of the NodeManager traits are included in the control flow only if compilation features are enabled. For this task we only need to the `node_tx` feature to control that the bundle is transmitted to the next node just after its arrival at the transmitting node, and we define a maximum treatment delay, for which a higher delay would be considered as retention:

```rust

#[cfg_attr(feature = "debug", derive(Debug))]
struct NoRetention {
    max_proc_time: Duration,
}

impl NodeManager for NoRetention {
    #[cfg(feature = "node_tx")]
    fn dry_run_tx(&self, waiting_since: Date, start: Date, _end: Date, _bundle: &Bundle) -> bool {
       return start - waiting_since < self.max_proc_time;
    }

    #[cfg(feature = "node_tx")]
    fn schedule_tx(
        &mut self,
        waiting_since: Date,
        start: Date,
        _end: Date,
        _bundle: &Bundle,
    ) -> bool {
       return start - waiting_since < self.max_proc_time;
    }

    // This manager only needs the node_tx feature
    // Those guards allow compilation even with the --all-features option
    #[cfg(feature = "node_proc")]
    fn dry_run_process(&self, _at_time: Date, _bundle: &mut Bundle) -> Date {
        panic!("Please disable the 'node_proc' and 'node_rx' features.");
    }

    #[cfg(feature = "node_proc")]
    fn schedule_process(&self, _at_time: Date, _bundle: &mut Bundle) -> Date {
        panic!("Please disable the 'node_proc' and 'node_rx' features.");
    }

    #[cfg(feature = "node_rx")]
    fn dry_run_rx(&self, _start: Date, _end: Date, _bundle: &Bundle) -> bool {
        panic!("Please disable the 'node_proc' and 'node_rx' features.");
    }
    #[cfg(feature = "node_rx")]
    fn schedule_rx(&mut self, _start: Date, _end: Date, _bundle: &Bundle) -> bool {
        panic!("Please disable the 'node_proc' and 'node_rx' features.");
    }
}
```


#### The parser

Now that the manager is ready, we can create elements of type `Node<NoRetention>` programmatically, but we are not yet ready to parse them from a contact plan. To do so, the `Parse<T>` trait must be implemented for `NoRetention`. This interface provides a `parse` class method that returns an element of type `T`, which we set to `NoRetention`. In other words, the library will be able to do `NoRetention::parse(...)` (internal machinery) to read the manager from the contact plan:


```rust
impl Parser<NoRetention> for NoRetention {
    fn parse(lexer: &mut dyn Lexer) -> ParsingState<NoRetention> {
        // read the next token as a Duration (alias for f64)
        let max = <Duration as Token<Duration>>::parse(lexer);
        // treat success/error cases
        match max {
            ParsingState::Finished(value) => {
                return ParsingState::Finished(NoRetention {
                    max_proc_time: value,
                })
            }
            ParsingState::Error(msg) => return ParsingState::Error(msg),
            ParsingState::EOF => {
                return ParsingState::Error(format!(
                    "Parsing failed ({})",
                    lexer.get_current_position()
                ))
            }
        }
    }
}
```


#### Dynamic parsing

If we want to have different management techniques for different nodes (e.g., we are not in a constellation, and only one node is unable to store bundles), we must implement the following trait:
```
impl DispatchParser<NoRetention> for NoRetention {}
```
The trait implementation is expected to remain empty, this will nevertheless activate the whole dispatching abilities.

The last step is to define a **marker** for our `NoRetention` manager, and because the markers are not hardcoded, we can just register this new marker/management technique with, this time, a `NodeMarkerMap`:

```
    let mut node_dispatch: NodeMarkerMap = NodeMarkerMap::new();
    node_dispatch.add("noret", coerce_nm::<NoRetention>);
    node_dispatch.add("none", coerce_nm::<NoManagement>);

```
