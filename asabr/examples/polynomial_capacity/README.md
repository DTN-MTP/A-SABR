## Polynomial Capacity Routing

### Run the example


```bash
cargo run --example polynomial_capacity
```

### Context

Standard SABR routing typically models contact capacities using constant data rates. However, in physical space networks, the actual throughput can vary dynamically during a pass (e.g., signal attenuation, elevation angle changes).

This feature introduces a polynomial-based capacity segmentation manager (`PolySegManager`). It allows modeling continuous, non-linear flow rates (such as parabolas) using exact integer-based integral mathematics. The mathematical engine is 100% `no_std` and avoids floating-point arithmetic, making it fully reliable for embedded systems and aerospace applications.

### Scenario

The network encompasses two nodes: Earth (0) and Mars (1).

A single contact between them is initialized statically using the `PolySegManager`.

The contact is characterized by a dynamic flow rate described by the polynomial $P(t) = 10 + 3t^2$ and a constant network delay of 4 units.

A single bundle of size 104 is scheduled for transmission at T=0.

### Behavior

It can be observed that the `PolySegManager` calculates the exact transmission end time without approximations. To find when the transferred volume reaches 104, it evaluates the exact integral of the flow rate: $F(t) = 10t + t^3$.

Instead of stepping through time linearly, the manager relies on a safe integer dichotomy (binary search) to find the exact root. For a bundle of size 104, the dichotomy perfectly converges to $t=4$ ($10(4) + 4^3 = 104$).

The final arrival time on Mars is correctly evaluated to T=8, securely adding the computed transmission end time (4) and the physical network delay (4).

### Implementation concerns

To prevent i64 overflow when evaluating high-degree polynomials over large time windows, the mathematical engine explicitly skips powers calculation for coefficients equal to zero.

Additionally, the maximum polynomial size is defined via a const generic N (e.g., 26), ensuring zero dynamic heap allocation for the coefficients array.