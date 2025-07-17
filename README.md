# Adaptive Schedule-Aware Bundle Routing

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE) [![Rust](https://img.shields.io/badge/rust-1.82%2B-orange.svg)](https://www.rust-lang.org)


Current version is a beta release (contact olivier.de-jonckere@lirmm.fr for more information). See source documentation [here](https://dtn-mtp.github.io/A-SABR/).

## Description

The A-SABR project provides a framework to instantiate routing algorithms from research activities up to operational contexts. This project was developed after the experience gathered from CGR at the Jet Propulsion Laboratory and the scalability research around Schedule-Aware Bundle Routing (SABR)'s scalability with SPSN at the University of Dresden.



**For researchers:** this framework aims to allow further routing algorithm development and benchmarking with a level of quality as close as possible to operational requirements.



**For operators:** built in Rust, the framework aims to reach the new recommendations regarding the use of memory-safe languages for future space missions. A-SABR uses polymorphism for composition and enforces the best performance whenever possible (by templating) and dynamic modularity if necessary (with dynamic dispatch), to compose its routing algorithms. Either directly compiled with the wanted component, or used as an inspiration to derive the future routing algorithm, A-SABR aims to accelerate the adoption of SABR in future operational activities.



**Built for flexibility and extensibility:** A-SABR is designed to exchange and add easily the building blocks of a routing algorithm. In some cases, variability can be desirable at runtime, in this case, multiple building block variations of the same type can be used simultaneously. For example, the earliest-transmission opportunity feature of SABR is applicable to the first hops, while the queue-delay feature takes over for the other hops.



The exchangeable building blocks are :

- Contact resource management (e.g. data rates, delays, volumes)

- Node resource management (e.g. processing, energy, transmission queues)

- Pathfinding algorithms (e.g. route construction and alternative route construction)

- Path storage (e.g. shortest-path trees, routes)

- Parsing capabilities to create flexible contact plans

- Distance calculation (e.g. SABR distance)

- Routing algorithms mainframes (e.g. CGR, SPSN)




## Mainframes and pathfinding

| **Algorithm Name**               | **Distance** | **Alternative Pathfinding** | **Dijkstra Variant** |
|----------------------------------|--------------|------------------------------|------------------------|
| SpsnHybridParenting                          | Sabr         | N/A                | HybridParenting                    |
| SpsnNodeParenting                    | Sabr         | N/A                | NodeParenting              |
| SpsnContactParenting                 | Sabr         | N/A                | ContactParenting           |
| CgrFirstEndingHybridParenting               | Sabr         | FirstEnding                  | HybridParenting                    |
| CgrFirstDepletedHybridParenting             | Sabr         | FirstDepleted                | HybridParenting                    |
| CgrFirstEndingNodeParenting         | Sabr         | FirstEnding                  | NodeParenting              |
| CgrFirstDepletedNodeParenting       | Sabr         | FirstDepleted                | NodeParenting              |
| CgrFirstEndingContactParenting      | Sabr         | FirstEnding                  | ContactParenting           |
| CgrFirstDepletedContactParenting    | Sabr         | FirstDepleted                | ContactParenting           |
| SpsnHybridParentingHop                       | Hop          | N/A                | HybridParenting                    |
| SpsnNodeParentingHop                | Hop          | N/A                | NodeParenting              |
| SpsnContactParentingHop             | Hop          | N/A                | ContactParenting           |
| CgrFirstEndingHybridParentingHop            | Hop          | FirstEnding                  | HybridParenting                    |
| CgrFirstDepletedHybridParentingHop          | Hop          | FirstDepleted                | HybridParenting                    |
| CgrFirstEndingNodeParentingHop      | Hop          | FirstEnding                  | NodeParenting              |
| CgrFirstDepletedNodeParentingHop    | Hop          | FirstDepleted                | NodeParenting              |
| CgrFirstEndingContactParentingHop   | Hop          | FirstEnding                  | ContactParenting           |
| CgrFirstDepletedContactParentingHop | Hop          | FirstDepleted                | ContactParenting           |
| VolCgrHybridParenting                        | Sabr         | N/A                | HybridParenting                    |
| VolCgrNodeParenting                 | Sabr         | N/A                | NodeParenting              |
| VolCgrContactParenting              | Sabr         | N/A                | ContactParenting           |
| VolCgrHybridParentingHop                    | Hop          | N/A                | HybridParenting                    |
| VolCgrNodeParentingHop              | Hop          | N/A                | NodeParenting              |
| VolCgrContactParentingHop           | Hop          | N/A                | ContactParenting           |

The Spsn based algorithms create shortest-path trees rather than single destination paths and consider the bundle metrics (priority and size) during tree computation to ensure at most one tree computation per bundle. A tree can be reused as long as the bundles to schedule show less constraining metrics (e.g. lower priority and smaller size) in comparison to the bundle metrics that were used to construct the present tree.

The Cgr based algorithms create single destination routes and do not consider the bundle metrics for path computation. Several path constructions might be required for a single bundle scheduling and they rely more extensively on route selection (as expected by the SABR standard).

The VolCgr based algorithms replace the alternative pathfinding approach with volume (and priority) aware search.

The algorithms are based on 3 pathfinding techniques (each of them declined in single-destination and shortest-path tree variants) :
- NodeParenting (or NodeGraph): Dijkstra with node to node tracking. Implementation mapping to the theoretical framework where nodes are vertices.
- ContactParenting (or ContactGraph): Dijkstra with contact to contact tracking, as in CGR. Implementation mapping to the theoretical framework where contacts are vertices.
- HybridParenting : Dijkstra with contact to contact tracking, tracking of multiple paths to individual node instead of direct overriding, and node based filtering.

And 2 alternative path strategies (for the Cgr mainframe):

- FirstEnding : Suppress first ending contact of the last found route before next computation.
- FirstDepleted : Suppress the contact with the smallest original volume limit before the next computation.

## Quick starts

This project includes several example programs demonstrating key features:

- **Contact Plans**: See [`examples/contact_plans/`](examples/contact_plans/) for contact plan formats and parsing.

- **Dijkstra Accuracy**: See [`examples/dijkstra_accuracy/`](examples/dijkstra_accuracy/) for the implementation of Dijkstra's algorithm accuracy tests.

- **Bundle Processing**: Check out [`examples/bundle_processing/`](examples/bundle_processing/) for bundle processing logic and related test cases.

- **ETO Management**: Explore [`examples/eto_management/`](examples/eto_management/) for managing Earliest Transmission Opportunity in the context of the library.

- **Satellite Constellation**: The satellite constellation example can be found in [`examples/satellite_constellation/`](examples/satellite_constellation/) to see how to implement a new resource management approach, to disable retention on nodes.


## Contact plans

Although wrappers are available to support existing formats (e.g. ION format, dtn-tvg-util), an A-SABR "native" format is leveraged to allow the addition of custom configuration capabilities for a new ```ContactManager```. Each contact plan source (file, stream, HTTP response, etc.) is managed by a ```Lexer``` creating tokens from this source. It's the lexer responsibility to manage eventual special characters (e.g. comment delimiters) and white spaces. Providing parsing capabilities to a component is translated by the implementation of a parsing trait, allowing the parsing logic to request tokens from the lexer in order to build the component.

A contact plan either provides "static" or "dynamic" contacts, referring to the dynamic dispatch ability if different contact or node manager types are assigned to different contacts (the dynamic behavior can be assigned to nodes or contact separately). If the contacts (or nodes) are parsed in dynamic mode, each contact (or node) entry must present a marker after the shared metrics.

## Contact management

10 volume management techniques are available.

#### Legacy

The first 9 approaches are similar enough to be generated with a unique macro. A "P" prefix means "with priority", and the "PB" prefix "with priorities and budgets". Budgeted priorities allow limiting the maximal volume that can be booked for a given priority level. The approaches listed below are already generated. The macro can be leveraged to create variants with a higher priority level count.

| **Manager** |**priority<br>levels** | **priority<br>budget** |
|-------------|---------------------|--------------------------------|
| EVLManager                                                   | 0                     | N/A                     |
| PEVLManager                                                  | 3                       | no                      |
| PBEVLManager                                                | 3                       | yes                     |
| ETOManager                                           | 0                     | N/A                     |
| PETOManager                                           | 3                       | no                      |
| PBETOManager                                         | 3                       | yes                     |
| QDManager                                     | 0                    | N/A                     |
| PQDManager                                    | 3                       | no                      |
| PBQDManager                                   | 3                       | yes                     |



- [P|PB]EVLmanager (Effective Volume Limit): tracking of the residual volume of the contacts.

- [P|PB]ETOmanager (Earliest Transmission Opportunity, for first hop contacts only): tracking of the transmission queue with a neighboring node. `IMPORTANT:` Real queue access would require huge coupling with the BPA, instead, manual queueing/dequeueing should be performed.

- [P|PB]QDManager (Queue Delay, an ETO variant for the next hops): tracking of the residual volume of the contacts, adds a delay for the earliest transmission opportunity from the contact start time depending on the booked volume (alternative to ETOManager for contacts that do not present the local node as transmitter).

The contact plan format will change for the budgeted versions.
```
# A-SABR CP Format for EVL/ETO/QD with or without priority (with marker if dynamic)
contact <from> <to> <start> <end> [marker] <rate> <delay>

# A-SABR CP Format for EVL/ETO/QD with priority (3 levels) **and** budget (with marker if dynamic)
contact <from> <to> <start> <end> [marker] <rate> <delay> <bugdet_1> <bugdet_2> <bugdet_3>
```
#### Contact Segmentation

The SegmentationManager tracks accurately the interval of bandwidth availability & utilization. It is suitable for any contact and can replace EVL, ETO and QD. When replacing ETO for segmentation, the performance is highly dependent on the contact plan accuracy, where ETO can be reactive to inaccuracies. In opposition to other approaches, a single logical contact can show different rates on different sub-intervals, where the physical contact would be split in 2 logical contacts for the legacy approaches. If a physical contact is split in two, a large bundle cannot overlap the two logical contacts during pathfinding/selection.

```
# A-SABR CP format for a segmented contact showing 2 intervals with different data rates
# but a single delay for its whole duration (with marker if dynamic)
contact <from> <to> <start> <end> [marker]
rate <start> <end> <rate>
rate <start> <end> <rate>
delay <start> <end> <delay>
```

## References
- EVL (Effective Volume Limit) : Blue Book, “Schedule-aware bundle routing,” Consultative Committee for Space Data Systems, 2019.
- ETO (Earliest Transmission Opportunity) : N. Bezirgiannidis, C. Caini, D. P. Montenero, M. Ruggieri, and V. Tsaoussidis, “Contact graph routing enhancements for delay tolerant space communications,” in 2014 7th advanced satellite multimedia systems conference and the 13th signal processing for space communications workshop (ASMS/SPSC). IEEE, 2014, pp. 17–23.
- Queue-delay : C. Caini, G. M. De Cola, and L. Persampieri, “Schedule-aware bundle routing: Analysis and enhancements,” International Journal of Satellite Communications and Networking, vol. 39, no. 3, pp. 237–249, 2021.
- Contact segmentation : De Jonckere, O., Fraire, J. A. A., & Burleigh, S. (2024). Distributed Volume Management in Space DTNs: Scoping Schedule-Aware Bundle Routing.
- FirstEnding & FirstDepleted : A. Fraire, P. G. Madoery, A. Charif, and J. M. Finochietto, “On route table computation strategies in delay-tolerant satellite networks,” Ad Hoc Networks, vol. 80, pp. 31–40, 2018
- HybridParenting (Formerly multipath-tracking) : O. De Jonckère, J. A. Fraire, and S. Burleigh, “Enhanced pathfinding and scalability with shortest-path tree routing for space networks,” in ICC 2023-IEEE International Conference on Communications. IEEE, 2023, pp. 4082–4088.
- Contact Graph Routing : J. A. Fraire, O. De Jonckère, and S. C. Burleigh, “Routing in the space internet: A contact graph routing tutorial,” Journal of Network and Computer Applications, vol. 174, p. 102884, 2021.
