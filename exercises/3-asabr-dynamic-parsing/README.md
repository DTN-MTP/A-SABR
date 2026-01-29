# Contact plan format flexibility

## Motivation

To archieve full flexibility, the format shall allow assigning different managers to different contacts or nodes, and this in the same contact plan. Indeed, the methods covered previously assigned a unique type to all contacts or nodes. The main motivation for this approach is to allow an assignment of managing techniques "à la carte", for example, `ETOManager` for uncertain first hop links, `ContactSegmentation` for highly accurate contacts, and `PBQDManager` for very important backbone links to apply maximum budget for lower priority bundles.

In SABR's specification, different contacts may have different managing techniques: relative to the local node, ETO (Earliest Transmission Opportunity) must be performed on the first hop contacts and EVL to the others. Implementation wise, this is most of the time hardcoded.

## Some more Rust concepts

### Dynamic dispatch to leverage interfaces

In the following example, we now want to store a list of concrete types implementing `Displayable`. Templating aligns the templated type in memory, so as long as the types implementing `Displayable` may have different sizes, only their addresses can be aligned. The Rust `Vec` is a C/C++-like dynamic array that aligns the elements in memory, and it is templated. If we want to store addresses (with `&` as in C) to concrete types implementing my trait, the `dyn` keyword will be required.

```rust
trait Displayable {
    fn display(&self);
}

struct TypeA {
    a_number: u32
}

impl Displayable for TypeA {
    fn display(&self){
        println!("{}", self.a_number)
    }
}

struct TypeB {
    a_string: String
}

impl Displayable for TypeB {
    fn display(&self){
        println!("{}", self.a_string)
    }
}


fn main() {
    let s1: TypeA = TypeA { a_number: 34 };
    let s2: TypeB = TypeB { a_string: String::from("hello!") };

    // A list of pointers to structures implementing "Displayable"
    let my_list: Vec<&dyn Displayable> = vec![&s1, &s2];
    for ele in &my_list {
        ele.display(); // dynamic dispatch
    }
}
```


### Dynamic allocation

All variables and structures are in the call stack by default. Heap allocation is implemented with templated types.

*Note: This does not necessarily require syscalls and a heap, the global allocator can be overridden to delegate allocation to a custom manager. Implementing SDR-like disk writes on modification would require slightly more effort.*

A **Boxed** value will be heap allocated :

```rust
struct TypeA {
    member: u32
}

fn main () {
    let heap_allocated: Box<TypeA> = Box::new(TypeA{member: 42});
}
```

The `Box` only allows single ownership of the data. In A-SABR's current version, we require shared ownership. For example, both the `Multigraph` and a `RouteStage` may require a reference to the same contact. The `Rc` allows shared ownership, and the `RefCell` provides a guard for interior mutability.

```rust
use std::{cell::RefCell, rc::Rc};

struct TypeA {
    member: u32
}

fn main () {
    let heap_allocated: Rc<RefCell<TypeA>> = Rc::new(RefCell::new(TypeA{member: 42}));

    // We open a block to constrain the scope of "borrowed"
    {
        let mut borrowed = heap_allocated.try_borrow_mut();
        borrowed.member = 43;
        // will panic, only one mutable borrow at a time
        // let mut illegal = heap_allocated.try_borrow_mut();
    } // borrowed leaves the scope
    let mut legal = heap_allocated.try_borrow_mut();
}
```

## Dynamic contact plans in A-SABR

#### Requirements

Relaxing this constraint in A-SABR requires two things:

1.  A support for one of those types:
    ```rust
    Contact<&mut dyn ContactManager>
    Contact<Rc<RefCell<dyn ContactManager>>>
    // A-SABR uses the following one:
    Contact<Box<dyn ContactManager>>
    ```
2.  A parsing technique and/or a contact plan format that allow to detect which manager is being parsed.

    A-SABR requires both for the simple reason that different managing techniques may have the same format. Indeed, in the exercise *0-asabr-static-parsing*, we could switch the template type from `EVLManager` to `QDManager` or `ETOManager` without modification of the contact plan file.

#### Manager type markers

To parse dynamically the contact plan will slightly change to add a **marker**:
```bash
# Static parsing
["contact"|"node"][base members][manager members]
# Dynamic parsing
["contact"|"node"][base members][marker][manager members]
```
In the following example, the **marker** type is used to distinguish between different contact manager implementations, `eto`, `evl`, and `qd` are the markers of `ETOManager`, `EVLManager`, and `QDManager`, respectively:
```
contact 0 1 60 7260 eto 10000 10
contact 1 2 60 7260 evl 15000 15
contact 2 3 60 7260 evl 20000 20
contact 3 4 60 7260 qd 25000 25
contact 4 5 60 7260 qd 30000 30
```

#### Marker maps

The last component is the map that links each marker to its manager type. To maintain extensibility and flexibility, this map is not hardcoded; instead, it must be provided by the user.

If one type of manager is parsed dynamically, then the corresponding dispatching map should be supplied to the parser as an `Option`. In the following example, template type for the node managers is `NoManagement`, the map `Option` for the nodes can remain `None`. However, the contact manager types will be indicated in the contact plan with **markers** for each contact, the dispatching map should be supplied

```rust
    let mut contact_dispatch: ContactMarkerMap = ContactMarkerMap::new();
    contact_dispatch.add("eto", coerce_cm::<ETOManager>);
    contact_dispatch.add("qd", coerce_cm::<QDManager>);
    contact_dispatch.add("evl", coerce_cm::<EVLManager>);

    let (nodes, contacts) = ASABRContactPlan::parse::<NoManagement, Box<dyn ContactManager>>(
        &mut mylexer,
        None, // No variability on nodes
        Some(&contact_dispatch), // Dispatch map for contacts
    )
```
