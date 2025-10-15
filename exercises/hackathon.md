# STINT 2025 hackathon: A-SABR integrations

You have the choice between working together on integrating **A-SABR with µD3TN in Python**, or **with Hardy in Rust**.

In both cases, we choose to integrate A-SABR in a **separate routing service**, and not as part of the BP implementation's codebase.

## An A-SABR routing service for µD3TN

**A-SABR as a separate routing service**: µD3TN exposes a Protobuf API over either a Unix or TCP socket, for exchanging Bundles, configuring EIDs, and routing.
This API is called the **Application Agent Protocol** (AAP) and an overview is [available here in the docs](https://d3tn.gitlab.io/ud3tn/aap20/) with an example interaction sequence diagram [down in the same page](https://d3tn.gitlab.io/ud3tn/aap20/#example-aap-20-interactions).

There is a Python library for using the AAP. A guide for building an AAP 2.0 client is [available here](https://d3tn.gitlab.io/ud3tn/development/how-to-build-an-aap20-client-in-python/).

A-SABR provides Python bindings: A-SABR is a Rust library; Python bindings wrap the Rust program around a Python interface.
These bindings / Python library is [available here](https://github.com/DTN-MTP/A-SABR-Python).

The goal is to **run µD3TN** ([deployment guide here](https://d3tn.gitlab.io/ud3tn/usage/build-and-run/)) and create a Python program that **provides routing services to µD3TN**, by using the AAP 2.0 library and the A-SABR Python bindings.

## An A-SABR routing service for Hardy

**A-SABR as a separate routing service**: Hardy has mechanisms for routing using an external routing provider.
Hardy's source code is [available here](https://github.com/ricktaylor/hardy).

A-SABR is a library written in Rust.

The goal is to **run Hardy** and create a Rust program that **provides routing services to Hardy**.
