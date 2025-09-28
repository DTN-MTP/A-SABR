# Parsing contact plans with A-SABR

This exercise showcases how A-SABR is able to parse different contact plans formats.

## Rust Basics

Rust is a low-level language like C that provides high-level concepts like C++, but with guarantees of memory safety. The macro system is much more capable, and the policy is safety by default. For example struct members will be private by default, all variables will be immutable by default, etc.

The A-SABR library relies on polymorphic features that allow compile-time and runtime flexibility. This section will go through some important concepts and how they are implemented in Rust.

#### Templates

Rust offers templating capabilities, similar to C++. The first use of a templated type will trigger at compile time the creation of the according variant. They are "monomorphized" during compilation, this is a **static dispatch**. The memory layout is similar to C, no pointer indirection. In the following example, the type `TypeB<TypeA>` (`TypeB` templated by `TypeA`) is aligned in memory, and is part of the call stack (no heap allocation):

```rust
struct TypeA{
    member: u32
}

struct TypeB<T> {
    member1: f64,
    member2: T
}

fn main() {
    let my_struct: TypeB<TypeA> = TypeB {
        member1: 3.14,
        member2: TypeA { member: 42 },
    };
}
```

#### Traits to implement interfaces

Rust does not provide inheritance (for good reasons), but provides the concept of **trait**, which are similar to the Java interfaces.

In this example we want `TypeB` to be templated by a type that implements a specific interface (`MyTrait`):

```rust
trait MyTrait {}

struct TypeA {
    member: u32
}

impl MyTrait for TypeA {}

// The concrete type "T" MUST implement the trait "MyTrait"
struct TypeB<T: MyTrait> {
    member1: f64,
    member2: T,
}

fn main() {
    let my_struct_1: TypeA = TypeA { member: 34 };
    let my_struct_2: TypeB<TypeA> = TypeB {
        member1: 3.14,
        member2: TypeA { member: 42 },
    };
}
```


## Nodes and Contacts in A-SABR

#### Nodes

The `Node` structure is templated with some type `NM`. `NM` must implement the `NodeManger` trait (dealt with later).

```rust
pub struct ContactInfo {
    pub tx_node: NodeID,
    pub rx_node: NodeID,
    pub start: Date,
    pub end: Date,
}

pub struct Node<NM: NodeManager> {
    pub info: NodeInfo,
    pub manager: NM,
}
```

#### Contacts

The `Contact` structure is templated with some type `CM`. `CM` must implement the `ContactManger` trait (dealt with later).

```rust
pub struct NodeInfo {
    pub id: NodeID,
    pub name: NodeName,
    pub excluded: bool,
}

// Note: This is a simplified version
pub struct Contact<CM: ContactManager> {
    pub info: ContactInfo,
    pub manager: CM,
}
```

## Contact plans

A contact plan is tuple of type ```(Vec<Node<NM>>, Vec<Contact<CM>>)``` created with to a source (most likely a file). For the external supported formats, the prototypes look like that:

```rust
    IONContactPlan::parse::<NM, CM>(filename: &str)
    TVGUtilContactPlan::parse::<NM, CM>(filename: &str)
```

They both return a `Result` that can be **unwrapped** to the contact plan tuple.

### Options and Results

`Option` is a templated type that allows to translate the fact that a variable can either have `Some(<value>)` or `None`. To access the value, we must **unwrap** the option to treat the two cases. To do so there are various ways, here are 3 of them:

```rust
fn main() {
    let my_opt: Option<u32> = Some(42);

    // With match
    match my_opt {
        Some(val) => println!("{}", val),
        None => println!("None"),
    }
    // With the "if let Some()" method
    if let Some(val) = my_opt {
       println!("{}", val);
    } else {
        println!("None");
    }

    // illegal, val is not in this scope
    // println!("{}", val);

    // With a guard, really good because...
    let Some(val) = my_opt else {
        println!("None");
        // the else bloc must interrupt the function
        return;
    };
    // ... val is in this scope !
    println!("{}", val);
}
```

The `Result` type works the same way, but must be also templated by an error type. `Ok()` replaces `Some()` and `Err()` will be used instead of `None`.

But with a `Result` you might want to keep the error (to log it for example) and the "let .. else" method won't suffice. Here is how to implement a guard:

```rust
fn main() {
    let my_res: Result<i32, &str> = Ok(-40);

    // I can already tell the type of the unwrapped value...
    let val: i32 = match my_res {
        Ok(val) => val,
        Err(err) => {
            println!("{}", err);
            // ...because assigning err to val is unreachable
            return;
        },
    };
    println!("{}", val);
}
```