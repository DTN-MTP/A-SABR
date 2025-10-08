# A-SABR contact plan format

## Motivation

A-SABR is designed to be extensible, and one main aspect is the ability to define new resource management for nodes and contacts. Some new resource management may require additional information from contact planning that must be embedded in the contact plans.

This requires contact plan format extensions with a registration system that does not require the library source to be modified. The following excercise will go through the base principles.

## Components of a node or a contact (*elements*)

**From the previous exercise:** We know from the Node and Contact source code, that any *element* is composed of a **base** being a `ContactInfo` or a `NodeInfo`, and a templated **manager** part being of type `CM` (implementing the `ContactManager` or `NodeManager` trait):

```rust

pub struct ContactInfo {
    pub tx_node: NodeID,
    pub rx_node: NodeID,
    pub start: Date,
    pub end: Date,
}
// node
pub struct Node<NM: NodeManager> {
    pub info: NodeInfo,
    pub manager: NM,
}

pub struct NodeInfo {
    pub id: NodeID,
    pub name: NodeName,
    pub excluded: bool,
}

// contact
// Note: This is a simplified version
pub struct Contact<CM: ContactManager> {
    pub info: ContactInfo,
    pub manager: CM,
}
```

## Parsing logic for the static case

An element entry in the the A-SABR format is of the form:
```
["contact"|"node"][base members][manager members]
```
For a Contact, the base part expects the members `<from> <to> <start> <end>`, and the EVLManager expects the members`<rate> <delay>`. Consequently, the A-SABR parser will expect for a Contact managed with EVL (type `Contact<EVLManager>`) the following format:
```
# contact <from> <to> <start> <end> <rate> <delay>
contact 0 1 60 7260 10000 10
```
The base part of a Node expects `<id> <name>`, and the dummy `NoManagement` techniques expects **no members**. A `Node<NoManagement>` will have the following format:
```
# node <id> <name>
node 0 node1
```
*Note: the `NoManagement` technique shows no overhead. Even if the associated method calls are not be removed after compiler optimizations, those calls would still be absent unless features from the library are enabled at compile time.*

The parser knows which **manager** types to expect thanks to the templating of the **parse** function:

```rust
ASABRContactPlan::parse::<NM, CM>(...);
```
For `IONContactPlan` and `TVGUtilContactPlan`

*Note: For `IONContactPlan` and `TVGUtilContactPlan` the templating served a slightly different machinery: ION and TVG do not rely on the A-SABR parser(s). Raw contacts are parsed, and templating is used to specify the target type for conversion.*

## Lexer

For flexibility and extensibility, the `ASABRContactPlan::parse` function does not take a file path as parameter directly. An intermediary `Lexer` component is required. The `new` function returns a `Result` containing the lexer in case of success:

```rust
    FileLexer::new(filename: &str)
```

The `ASABRContactPlan` takes the lexer as parameter, but also two `Option` that can be set to `None` for the moment (more on that later).

```rust
    ASABRContactPlan::parse::<NoManagement, EVLManager>(mylexer, None, None)
```