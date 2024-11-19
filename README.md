
# Adaptive Schedule-Aware Bundle Routing [![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](LICENSE) [![Rust](https://img.shields.io/badge/rust-1.82%2B-orange.svg)](https://www.rust-lang.org)

## Description

See documentation [here](https://maore-dtn.lirmm.net/a-sabr/a_sabr)

The A-SABR project provides a framework to instantiate routing algorithms from research activities up to operational contexts. This project was developed after the experience gathered from CGR at the Jet Propulsion Laboratory and the scalability researches around Schedule-Aware Bundle Routing (SABR)'s scalability with SPSN at the University of Dresden.



**For researchers:** this framework aims to allow further routing algorithm development and benchmarking with a level of quality as close as possible to operational requirements.



**For operators:** built in Rust, the framework aims to reach the new recommendations regarding the use of memory-safe languages for future space missions. A-SABR uses polymorphism for composition and enforces the best performance whenever possible (by templating) and dynamic modularity if necessary (with dynamic dispatch), to compose its routing algorithms. Either directly compiled with the wanted component, or used as an inspiration for derive the future routing algorithm, A-SABR aims to accelerate the adoption of SABR in future operational activities.



**Built for flexibility and extensibility:** A-SABR is designed to exchange and add easily the building blocks of a routing algorithm. In some cases, variability can be desirable at runtime, in this case, multiple building blocks variations of the same type can be used simultaneously. For example, the earliest-transmission opportunity feature of SABR is applicable to the first hops, while the queue-delay feature takes over for the other hops.



The exchangeable building blocks are :

- Contact resource management (e.g. data rates, delays, volumes)

- Node resource management (e.g. processing, energy, transmission queues)

- Pathfinding algorithms (e.g. route construction and alternative route construction)

- Path storage (e.g. shortest-path trees, routes)

- Parsing capabilities to create flexible contact plans

- Distance calculation (e.g. SABR distance)

- Routing algorithms mainframes (e.g. CGR, SPSN)



A-SABR provides a list of already composed algorithms :

- SpsnMpt

- SpsnNodeGraph

- SpsnContactGraph

- CgrFirstEndingMpt

- CgrFirstDepletedMpt

- CgrFirstEndingNodeGraph

- CgrFirstDepletedNodeGraph

- CgrFirstEndingContactGraph

- CgrFirstDepletedContactGraph

- SpsnHopMpt

- SpsnHopNodeGraph

- SpsnHopContactGraph

- CgrHopFirstEndingMpt

- CgrHopFirstDepletedMpt

- CgrHopFirstEndingNodeGraph

- CgrHopFirstDepletedNodeGraph

- CgrHopFirstEndingContactGraph

- CgrHopFirstDepletedContactGraph

## Not only SABR

The types that represent progression of pathfinding algorithms as well as the final routes are templated with the ```Distance``` trait. Concrete implementations of the Distance trait act as proxies for distance comparison between the routes, called ```RouteStage```. For compatibility with all pathfinding techniques, the contacts are consequently templated the ```Distance``` trait, by carrying a ```RouteStage``` for contact graph (or contact parenting) pathfinding. A ```RouteStage``` is conceptually equivalent to contact work areas of ION.

___Performance note___ : the work area is attached to the contact if and only if the **contact_graph** compilation feature is enabled. Without this feature, no memory overhead remains as the ```Distance``` concrete types are usually implemented as empty structures (see ```SABR``` and ```Hop``` distance implementations).

## Contacts, Nodes, and Contact Plans

#### About Nodes

The parsing, dispatching, and resource managing concepts described in this section are applicable to nodes with the according trait ```NodeManager```. The nodes shall also present an internal NodeID (see sections **contact plans** and **pathfinding**)

___Performance note___ : When using the ```NoNodeManagement``` concrete implementation to disable node management logic, it is encouraged to use the **disable_node_management** compilation feature to entirely remove the control flow associated with node management.

#### Defining new managers

A main feature of A-SABR is the ability to exchange the volume management technique for the contacts. A contact exhibits a static part ```info``` (start time, end time, transmitter, and receiver), and a template part ```manager``` of type ```<CM: ContactManager>```. This type parameter can be :

- A type implementing the ```ContactManager``` trait (or interface), e.g. ```EVLManager```.

- A boxed type implementing the ```ContactManager``` trait, e.g. ```Box<EVLManager>```

- A boxed ```ContactManager``` to allow different management techniques from one contact to another, i.e ```Box<dyn ContactManager>```


Example of a ```NewManager``` implementation :

```rust
impl  ContactManager  for  NewManager {
  fn  dry_run(
    &self,
    contact_data: &ContactInfo,
    at_time: Date,
    bundle: &Bundle,
    ) -> Option<TxEndHopData> {
      // Simulate here the transmission and return a TxEndHopData
      // if transmission is possible
  }

  fn  schedule(
    &mut  self,
    contact_data: &ContactInfo,
    at_time: Date,
    bundle: &Bundle,
    ) -> Option<TxEndHopData> {
      // Apply here the transmission and return a TxEndHopData
      // after you modified the resources of this contact
      // The TxEndHopData MUST match the one that would be returned by
      // the dry run for the same inputs
      // Do not use this function if you didn't do a dry run just before
      // its call, with the same inputs
  }

  fn  try_init(&mut  self, contact_data: &ContactInfo) -> bool {
    // finalize the initialisation and sanity check with the
    // contact's information
  }

  #[cfg(feature = "first_depleted")]
  fn  get_original_volume(&self) -> Volume {
    // Implement this method if you intend to use the first_depleted
    // pathfinding algorithm
  }
}
```

#### New managers and contact plan format

##### Format A-SABR

This exchangeable part might need specific configuration for its initialization. Although wrappers are planned to support existing formats (e.g. ION format), an A-SABR "native" format is leveraged to allow the addition of custom configuration capabilities for a new ```ContactManager```. Each a contact plan source (file, stream, HTTP response, etc.) is managed by a ```Lexer``` which convert the source into tokens, it's the lexer responsibility to manage eventual special characters (e.g. comment delimiters) and white spaces. Providing parsing capabilities to a component is translated by the implementation of a parsing trait, allowing the parsing logic to request tokens from the lexer in order to build the component.

```rust
impl  Parser<NewManager> for  NewManager {
  fn  parse(lexer: &mut  dyn  crate::parsing::Lexer) -> ParsingState<NewManager> {
    // Logic example if the manager only cares of the contact propagation delay
    // It is inialized with a delay value
    match  Duration::parse(lexer) {
      ParsingState::Finished(delay) => return  ParsingState::Finished(NewManager::new(delay)),
      ParsingState::Error(msg) => return  ParsingState::Error(msg),
      ParsingState::EOF => {
        return  crate::parsing::ParsingState::Error(format!(
          "Unexpected EOF ({})",
          lexer.get_current_position()
        ))
      }
    }
  }
}
```

#### Contact plans

Once the new managers are defined with implementations of the ContactManager and Parser traits, a "native" contact plan format can be derived from the various manager implementations. We will consider the example ```NewManager```, and the library-provided ```ContactSegmentation```.

If you wish to use a unique manager types (e.g. a **boxed**  ```NewManager``` for the contacts and ```NoManagement``` for the nodes) no other action is required for dispatching, and you can extract the nodes and contacts from a lexer. The format of the contact consists of the shared metrics concatenated with the metrics defined by the Parser implemention (for ```NewManager```, a ```Delay``` value is expected).

The ```FileLexer``` forces you to declare the node names, and will work with the following parsing logic :

```rust
if  let  Ok(mut  mylexer) = FileLexer::new("/path/to/asabr_cp.txt") {
  let  mut  cp = ContactPlan::new();
  let  res = cp.parse::<NoManagement, Box<NewManager>>(&mut  mylexer, None, None);
  // The type of res is : Result<(Vec<Node<NoManagement>, Vec<Contact<Box<NewManager>>>)>
}

```

Expecting a file of the following format :

```text
# The NoManagement does not expect anything after the shared metrics
# The node format will only require the shared metrics as follows :
# node <NodeID> <Alias>
node 0 gs1
node 1 gs2
node 2 sat1
node 3 sat2

# A delay is expected after the shared metrics, the format is :
# contact <tx_node> <rx_node> <start> <end> <delay>
contact 1 2 30 40 0
contact 2 3 50 70 3
# etc.
```

If having contacts of different types in the same contact plan is desired to optimize the performance on a single contact basis, the contact plan requires a marker to dispatch to the correct parser :

```rust
let  mut  cm_map: Dispatcher<ContactDispatcher> = Dispatcher::<ContactDispatcher>::new();
cm_map.add("new", coerce_cm::<NewManager>);
cm_map.add("seg", coerce_cm::<SegmentationManager>);

if  let  Ok(mut  mylexer) = FileLexer::new("/path/to/asabr_cp.txt") {
  let  mut  cp = ContactPlan::new();
  let  res = cp.parse::<NoManagement, Box<dyn  ContactManager>>(&mut  mylexer, None, Some(cm_map));
  // The type of res is : Result<(Vec<Node<NoManagement>, Vec<Contact<Box<dyn ContactManager>>>)>
}
```

The dispatching capabilities must also be enabled by implementing the following trait :

```rust
impl  DispatchParser<NewManager> for  NewManager {}
```

The contact plan can now encompass contacts of different types, distinguished by their respective markers (here ```new``` and ```seg```) :

```text
# The node format does not change
node 0 gs1
node 1 gs2
node 2 sat1
node 3 sat2

# A marker is now expected after the shared metrics
# For the NewManager the marker is "new"
contact 1 2 30 40 new 0
contact 2 3 50 70 new 3
# etc.

# A segmented contact showing 2 intervals with different data rates
# but the same delay for its entire duration

contact 3 2 300 400 seg
rate 300 350 9600
rate 350 400 9600
delay 300 400 1
```

##### ION and tvg-util formats

Two other format are supported for file sources. Those parsers do not variability, the NodeManager is set to ```NoManagement```, and the ```ContactManager``` of the contacts will be of the same type. Future work may include the initialization of a given manager for the contacts to the direct neighbors and another manager type for the other contacts (e.g. ETO and EVL). Those formats are provided for convenience, use A-SABR for maximal flexibility. Inialization example :


```rust
    if let Ok((nodes, contacts)) IONContactPlan::parse::<SegmentationManager>("cp_examples/ion_cp.txt") {
      // ...
    }

    if let Ok((nodes, contacts)) = TVGUtilContactPlan::parse::<EVLManager>("cp_examples/tvgutil_cp.json") {
      // ...
    }
```

## Multigraph

In A-SABR, the terms "contact graph" and "node graph" are used for convenience as **pathfinding denominations**, but do not refer to data structures in any ways. The contacts are instead stored in a ```Multigraph``` structure regardless of the pathfinding technique, and has the sole role of providing optimized contact access. Contact access is *close** to O(1), while the use of a RB-Tree (being a sorted list implementation) would require a cursor positioning of O(log\(C\)), C being the contact count. The contacts to a receiver are sorted by start times, and the provided pathfinding implementations support contacts that overlap in time between two nodes. This feature is enabled to provide more flexibility to contact planning and possible future contact selection criteria for pathfinding.

\* : The multigraph embeds a "lazy pruning" mechanism that maintains an index value to the first relevant contact (i.e. non expired) to each receiver from a given transmitter. The NodeIDs are also used as array indices to avoid hash function calls associated with multigraph implementations using dictionaries/maps.

## Pathfinding, route storage, and routing algorithms

#### Pathfinding

A pathfinding algorithm implements the ```Pathfinding``` trait that provides a method to get the next ```PathfindingOutput``` Output (tree or route). A pathfinding algorithm can either be a shortest-path finding algorithm (e.g. a variant of Dijkstra like contact graph, node graph or mpt) or an alternative pathfinding algorithm (e.g. Yen, first-ending, or first-depleted). The latter depending on the former, the Pathfinding algorithm can be composed prior to their utilization in the routing algorithm (e.g. the ```CgrFirstDepletedNodeGraph``` routing algorithm uses the first-depleted alternative pathfinding approach with a node graph variant of Dijkstra as backend).

#### Route storage

To lower the coupling and lower memory overhead, the framework enforces a differentiation between single destination paths, or route and shortest-path tree when it comes to their storage. Two storage approaches are part of this first version :

-  ```TreeCache``` : it stores one tree per node exclusion list. If the exclusion list are destination based, having a cache size equal to the number of destinations provides full capabilities. You can also reduce the cache size to reduce the memory pressure. With no congestion, maintaining a single tree is often the best option.

-  ```RoutingTable``` : Single-destination lists of routes, with best candidate election as described in SABR.

## Full Example

Initialization and routing with an ASABR contact plan using ```ETOManager``` and ```SegmentationManager``` types of ```ContactManager```.


```rust
let mut contact_dispatch: Dispatcher<ContactDispatcher> =
    Dispatcher::<ContactDispatcher>::new();
contact_dispatch.add("eto", coerce_cm::<ETOManager>);
contact_dispatch.add("seg", coerce_cm::<SegmentationManager>);

if let Ok(mut mylexer) = FileLexer::new("/path/to/asabr_cp.txt") {
  let mut cp = ASABRContactPlan::new();
  let res = cp.parse::<NoManagement, Box<dyn ContactManager>>(&mut mylexer, None, Some(&contact_dispatch));

  if let Ok((nodes, contacts)) = res{
    {
      let tree_cache = Rc::new(RefCell::new(TreeCache::new(true, false, 10)));
      let mut SPSN = SpsnSabrContactGraph::<NoManagement, Box<dyn ContactManager>>::new(nodes, contacts, tree_cache, false);
      let multicast_bundle = Bundle {
        source: 0,
        destinations: vec![1, 2, 3, 4],
        priority: 0,
        size: 1.0,
        expiration: Date::MAX,
      };
      let exclusions = Vec::new();
      let first_hops = SPSN.route(0, &multicast_bundle, 0.0, &exclusions);
    }
  }
}

```

## Current limitations

Increasing the coupling for flexibility can create some overhead (e.g. with extra control flow or memory pressure of the structure). In order to stay as close as possible to what shall be expected for operational performance, the coupling is reduced in two ways :

- By design,  with the effect of lowering slightly the flexibility (e.g. with the differenciation of trees and routes for storage). In some case, this can prevent compositions between uncompatible building blocks.

- With compilation features, with the effect of requiring a recompilation of the library to use some algorithms with maximal performance. If recompilation of the library is not an option, all features can be enabled : some unecessary overhead is expected for the memory pressure (e.g. with contacts carrying a work area even for non contact graph pathfinding) and control flow (e.g. calls to node management functions that have no effects with the ```NoManagement``` concrete implementation).
