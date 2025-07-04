## Contact plans


### Run the example

This example requires the `contact_work_area` feature:
```bash
cargo run --example contact_plans
```

### Principles

A routing algorithm operates with a multigraph, and the multigraph shall be initialized with nodes and contacts retrieved from a source (e.g. a contact plan file). In A-SABR, the nodes and contacts are initialized with a manager, which is attached to the contact. The overhead can be null if the entry (node or contact) is parsed "statically", in this case, the manager will be aligned in memory.

Indeed, the node & contact entry set can either be static (for all contact or node entries, and they must share the same manager type), or dynamically (node or contact entries can have managers of different types). The static/dynamic naming comes from the need to involve dynamic dispatch at runtime. If the contacts are parsed dynamically, the managers are heap allocated, and a contact holds a reference to its manager.

### ION & TVG-UTIL

For ION & TVG-UTIL, node resource management is not supported (as not part of the SABR standard), and all the contacts must be of the same type (static parsing). Future work may allow assigning ETO managers to the first-hop contacts, with another approach for the others. The only sources available for parsing are contact plan files.

If a user creates a new contact manager, but does not want to use the A-SABR contact plan format, the new manager can be available for parsing by implementing the traits `FromIONContactData` or `FromTVGUtilContactData` for this new manager.

### A-SABR Format

Parsing a A-SABR contact plan allows for variability on the nodes or contacts. Each entry (node or contact) presents a shared part, and a manager-specific part. When parsing a contact plan statically, there are no extra requirements.

When parsing an entry type dynamically (node or contact), each entry of type must present a marker between the shared part and the manager part, for parsing dispatch. In this case, a dispatching map must be provided to the parsing function.

Last but not least, a A-SABR contact plan requires a `Lexer` that tokenizes a source for the parser. This allows the support of other sources or formats (e.g. json). The creation of a new manager is out of scope of this example.