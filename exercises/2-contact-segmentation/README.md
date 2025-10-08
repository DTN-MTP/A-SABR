# Contact segmentation

## Motivation

**Contact segmentation** is a volume management technique that provides for a single contact:
- Time segmentation of the delay intervals
- Time segmentation of the rate intervals
- Time segmentation of the available bandwidth intervals

In other formats, such a **physical contact** would be converted in several **logical contacts**. With ION, this would be translated to several contacts, one for each data rate interval and several range messages to translate the variation of link delay.

Contact segmentation allows maintaining a **single logical** contact for each **physical contact**. At first glance, this may look like nothing more than a software design convenience, but in fact it was motivated by real operational challenges:
- **Large bundles and fragmentation**: Bundles that could be transmitted without fragmentation on a physical contact might overlap two or more logical contacts. Without segmentation, such bundles risk either being fragmented unnecessarily or scheduled on an alternate route.
- **Time-slot booking vs. volume reduction**: Scheduling a bundle on a contact’s route is less about decrementing the EVL (residual volume) and more about reserving a specific time interval within the contact’s duration for transmission.
- **Replacing legacy metrics with robustness**: If the contact plan is accurate, segmentation can be applied to each contact to replace metrics like ETO and EVL, while offering greater robustness and precision:
    - When subsequent contacts overlap in time along an end-to-end path, EVL falls short.
    - When bundles are very large, ETO falls short.
    - When bandwidth utilization is highly variable, QD falls short.


## ION and TVGUtil formats

The ION and TVGUtil format can be parsed templated by the SegmentationManager.

The translation is however very naive and the fact that several **logical contacts** are in fact a single **physical contact** won't be detected, and one `Contact<SegmentationManager>` per **logical contact** will be created. We still can leverage the segmentation of available bandwidth intervals.


## A-SABR format

The SegmentationManager parser expects a serie of delay/rate intervals or the form `delay <start> <end> <delay>` and `rate <start> <end> <rate>`.

Here is a contact from node 0 to node 5 between t=0 and t=7260 encompassing a single delay value for its whole duration (12) but a varying data rate (10000 from t=0 until t=3600 and 15000 from t=3600 until t=7260)

```bash
contact 0 5 60 7260 rate 60 3660 10000 rate 3660 7260 15000 delay 60 7260 12
```
A-SABR format does not care of whitespaces, including new lines, here is a nice formating:

```bash
contact 0 5 60 7260
   rate 60 3660 10000
   rate 3660 7260 15000
   delay 60 7260 12
```

*Note: A-SABR currently requires node IDs to be within the range [0, node_count - 1]. This constraint might be lifted in future versions.*

