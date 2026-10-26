# Adaptive Schedule-Aware Bundle Routing

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE) [![Rust](https://img.shields.io/badge/rust-1.82%2B-orange.svg)](https://www.rust-lang.org)


Current version is a beta release (contact olivier.de-jonckere@lirmm.fr for more information). See source documentation [here](https://dtn-mtp.github.io/A-SABR/).

## Description

The A-SABR project provides a framework to instantiate routing algorithms from research activities
up to operational contexts. This project was developed after the experience gathered from CGR at the
Jet Propulsion Laboratory and the scalability research around Schedule-Aware Bundle Routing (SABR)'s
scalability with SPSN at the University of Dresden.


**For researchers:** this framework aims to allow further routing algorithm development
and benchmarking with a level of quality as close as possible to operational requirements.


**For operators:** built in Rust, the framework aims to reach the new recommendations regarding
the use of memory-safe languages for future space missions. A-SABR uses polymorphism for composition
and enforces the best performance whenever possible (by templating) and dynamic modularity if necessary
(with dynamic dispatch), to compose its routing algorithms. Either directly compiled with the wanted component,
or used as an inspiration to derive the future routing algorithm, A-SABR aims to accelerate the adoption
of SABR in future operational activities.


**Built for flexibility and extensibility:** A-SABR is designed to exchange and add easily the building blocks
of a routing algorithm. In some cases, variability can be desirable at runtime, in this case, multiple building
block variations of the same type can be used simultaneously. For example, the earliest-transmission opportunity
feature of SABR is applicable to the first hops, while the queue-delay feature takes over for the other hops.



The exchangeable building blocks are :

- Contact resource management (e.g. data rates, delays, volumes)

- Node resource management (e.g. processing, energy, transmission queues)

- Pathfinding algorithms (e.g. bare dijkstra, crg, spsn)

- Path storage (e.g. shortest-path trees, routes)

- Parsing capabilities to create flexible contact plans

- Distance calculation (e.g. SABR distance)



## Classicals Pathfindings algorithms

It is possible to use custom combination of pathfinding algorithms and distances, but some classical combinations have been given names in
pathfinding::alias.

Each combination name is the concatenation of the Algorithm Name, Variant, Dijkstra impl, and eventually "Hop" to change from the default SABR distance to an
hop-first distance.

### Available algorithms:
- Spsn (Recommended): Work on shortest-path tree for the whole graph, and consider bundle metrics to create them to avoid multiple tree computation.
  - No Variants
- VolCgr: Work on shortest-paths directly, and consider bundle metrics to create them to avoid multiple tree computation.
  - No Variants
- Cgr: Try to construct paths without considering bundle metrics, blacklisting contacts on the computed path one by one as long as it does not work.
  - FirstDepleted: The blacklisted contact is the one with least initial capacity
  - FirstEnding: The blacklisted contact is the one which stop existing first

### Available disjkstra implementations (Can be used directly):
- HybridParenting (Recommended) : Explore for each node, but allow re-exploring for non-yet-optimal paths. Most accurate, and fast in most practical cases.
- ContactParenting : Explore the best path to each contact in the graph. Slowest
- NodeParenting : Explore only the best path to each node in the graph. The fastest but less accurate

### Distances
- SABR (Recommended, not specified in the aliases): Lexicographical order on (time,number of hop,expiration)
- Hop : Lexicographical order on (number of hop,time,expiration). May be faster to calculate best routes for

## Quick starts

This project includes several example programs demonstrating key features:

- **Contact Plans**: See [`asabr/examples/contact_plans/`](asabr/examples/contact_plans/) for contact plan formats and parsing.

- **Dijkstra Accuracy**: See [`asabr/examples/dijkstra_accuracy/`](asabr/examples/dijkstra_accuracy/) for the implementation of Dijkstra's algorithm accuracy tests.

- **ETO Management**: Explore [`asabr/examples/eto_management/`](asabr/examples/eto_management/) for managing Earliest Transmission Opportunity in the context of the library.

- **Satellite Constellation**: The satellite constellation example can be found in [`asabr/examples/satellite_constellation/`](asabr/examples/satellite_constellation/) to see how to implement a new resource management approach, to disable retention on nodes.


## Contact plans

Although wrappers are available to support existing formats (e.g. ION format, dtn-tvg-util), an A-SABR "native" format is leveraged to allow the addition of custom configuration capabilities for a new ```ContactManager```. Each contact plan source (file, stream, HTTP response, etc.) is managed by a ```Lexer``` creating tokens from this source. It's the lexer responsibility to manage eventual special characters (e.g. comment delimiters) and white spaces. Providing parsing capabilities to a component is translated by the implementation of a parsing trait, allowing the parsing logic to request tokens from the lexer in order to build the component.

A contact plan either provides "static" or "dynamic" contacts, referring to the dynamic dispatch ability if different contact or node manager types are assigned to different contacts (the dynamic behavior can be assigned to nodes or contact separately). If the contacts (or nodes) are parsed in dynamic mode, each contact (or node) entry must present a marker after the shared metrics.

## Contact management

11 volume management techniques are available.

#### Legacy

3 Management technique, each with tree variants:


- [P|PB]EVLmanager (Effective Volume Limit): tracking of the residual total volume of the contact.

- [P|PB]ETOmanager (Earliest Transmission Opportunity, for first hop contacts only): tracking of the transmission queue with a neighboring node.
    `IMPORTANT:` Real queue access would require huge coupling with the BPA, instead, manual queueing/dequeueing should be performed to mirror the action on the real queue.

- [P|PB]QDManager (Queue Delay, an ETO variant for the next hops): tracking of the residual volume of the contacts, adds a delay for the earliest transmission opportunity from the contact start time depending on the booked volume (alternative to ETOManager for contacts that do not present the local node as transmitter).

"P" prefix means "with priority", and the "PB" prefix "with priorities and budgets per priority".
Budgeted priorities allow limiting the maximal volume that can be booked for a given priority level.

The P and PB variants use a maximum priority of 3, (any more will be considered as priority level 3).

If a different number of priority is desired, use `contact_manager::legacy::Legacy` directly.

The contact plan format will change for the budgeted versions.
```
# A-SABR CP Format for EVL/ETO/QD with or without priority (with marker if dynamic)
contact <from> <to> <start> <end> [marker] <rate> <delay>

# A-SABR CP Format for EVL/ETO/QD with priority (3 levels) **and** budget (with marker if dynamic)
contact <from> <to> <start> <end> [marker] <rate> <delay> <bugdet_1> <bugdet_2> <bugdet_3>
```

#### Contact Segmentation

The [P]SegmentationManager tracks accurately the interval of bandwidth availability & utilization.

It is suitable for any contact and can replace EVL, ETO and QD.
When replacing ETO for segmentation, the performance is highly dependent on the contact plan accuracy, where ETO can be reactive to inaccuracies.
In opposition to other approaches, a single logical contact can show different rates on different sub-intervals, where the physical contact would
be split in 2 logical contacts for the legacy approaches. If a physical contact is split in two, a large bundle cannot overlap the two logical
contacts during pathfinding/selection.

The P variant is priority aware.

```
# A-SABR CP format for a segmented contact showing 2 intervals with different data rates
# but a single delay for its whole duration (with marker if dynamic)
contact <from> <to> <start> <end> [marker]
rate [<start> <end> <rate>, ...(repeat) ]
delay [ <start> <end> <delay>, ...(repeat) ]
```

## References
- EVL (Effective Volume Limit) : Blue Book, “Schedule-aware bundle routing,” Consultative Committee for Space Data Systems, 2019.
- ETO (Earliest Transmission Opportunity) : N. Bezirgiannidis, C. Caini, D. P. Montenero, M. Ruggieri, and V. Tsaoussidis, “Contact graph routing enhancements for delay tolerant space communications,” in 2014 7th advanced satellite multimedia systems conference and the 13th signal processing for space communications workshop (ASMS/SPSC). IEEE, 2014, pp. 17–23.
- Queue-delay : C. Caini, G. M. De Cola, and L. Persampieri, “Schedule-aware bundle routing: Analysis and enhancements,” International Journal of Satellite Communications and Networking, vol. 39, no. 3, pp. 237–249, 2021.
- Contact segmentation : De Jonckere, O., Fraire, J. A. A., & Burleigh, S. (2024). Distributed Volume Management in Space DTNs: Scoping Schedule-Aware Bundle Routing.
- FirstEnding & FirstDepleted : A. Fraire, P. G. Madoery, A. Charif, and J. M. Finochietto, “On route table computation strategies in delay-tolerant satellite networks,” Ad Hoc Networks, vol. 80, pp. 31–40, 2018
- HybridParenting (Formerly multipath-tracking) : O. De Jonckère, J. A. Fraire, and S. Burleigh, “Enhanced pathfinding and scalability with shortest-path tree routing for space networks,” in ICC 2023-IEEE International Conference on Communications. IEEE, 2023, pp. 4082–4088.
- Contact Graph Routing : J. A. Fraire, O. De Jonckère, and S. C. Burleigh, “Routing in the space internet: A contact graph routing tutorial,” Journal of Network and Computer Applications, vol. 174, p. 102884, 2021.
