# STINT Hackathon

## Getting started

Rust is a demanding (but rewarding) language, we encourage the use of an IDE with syntax analysis, for example with VScode and the **rust-analyzer** extension. And you can install Rust from [rust-lang.org](https://rust-lang.org/tools/install/).

The library relies on compilation features, and you may get false negative errors or warnings in VScode from the analyzer. You can create a configuration file in the root folder of the repository to allow the analyzer to consider all features by default:

```bash
mkdir -p .vscode
cat > .vscode/settings.json << EOF
{
    "rust-analyzer.linkedProjects": [
        "./Cargo.toml"
    ],
    "rust-analyzer.cargo.allFeatures": true
}
EOF
```

From this point, we should be able to compile and run the exercises. Compiling and running an example can be done with the following command:

```bash
cargo run --example <example-name>
```

Each example consists of a folder that includes a \<example-name\>.rs that can be modified for the exercise. And example can also present a README.md for context, as well as contact plans files.


## Rust Basics

Rust is a low-level language like C that provides high-level concepts like C++, but with guarantees of memory safety. The macro system is much more capable, and the policy is safety by default. For example struct members will be private by default, all variables will be immutable by default, etc.

The library also relies on polymorphic features that allow compile-time and runtime flexibility. This section will go through some important concepts and how they are implemented in Rust.



### Templates and Static dispatch

Rust offers templating capabilities, similar to C++. The first use of a templated type will trigger at compile time the creation of the according variant (they are "monomorphized" during compilation). The memory layout is similar to C, no pointer indirection. In the following example, the type `TypeB<TypeA>` (`TypeB` templated by `TypeA`) is aligned in memory, and is part of the call stack (no heap allocation):

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

### Traits to implement interfaces

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

### Dynamic dispatch to leverage interfaces

In the following example, we now want to store a list of concrete types implementing `MyTrait`. Templating aligns the templated type in memory, so as long as the types implementing `MyTrait` may have different sizes, only their addresses can be aligned. The Rust Vector (`Vec`) is a C/C++-like dynamic array that aligns the elements in memory and is therefore a templated type. If we want to store addresses to concrete types implementing my trait, the `dyn` keyword will be required.

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

impl<T: MyTrait> MyTrait for TypeB<T> {}

fn main() {
    let my_struct_1: TypeA = TypeA { member: 34 };
    let my_struct_2: TypeB<TypeA> = TypeB {
        member1: 3.14,
        member2: TypeA { member: 42 },
    };

    // A list of pointers to structures implementing "MyTrait"
    let my_list: Vec<&dyn MyTrait> = vec![&my_struct_1, &my_struct_2];
}
```

### Dynamic allocation

All variables and structures are in the stack by default. Heap allocation is implemented with templated types.

A Boxed value will be heap allocated :

```rust
struct TypeA {
    member: u32
}

fn main () {
    let heap_allocated: Box<TypeA> = Box::new(TypeA{member: 42});
}
```

The `Box` only allows single ownership of the data. In A-SABR's current version, we require shared ownership. For example, both the `Multigraph` and a `RouteStage` may require a reference to the same contact. The `Rc` allow shared ownership, and the `RefCell` provides a guard for interior mutability.

```rust
use std::{cell::RefCell, rc::Rc};

struct TypeA {
    member: u32
}

fn main () {
    let heap_allocated: Rc<RefCell<TypeA>> = Rc::new(RefCell::new(TypeA{member: 42}));

    // We open a block to constrain the scope of "borrowed"
    {
        let mut borrowed = heap_allocated.borrow_mut();
        borrowed.member = 43;
        // will panic, only one mutable borrow a a time
        // let mut illegal = heap_allocated.borrow_mut();
    } // borrowed leaves the scope
    let mut legal = heap_allocated.borrow_mut();
}
```

### Options

`Option` is a templated type that allows to translate the fact that a variable can either have `Some` value or `None`. To access the value, we must **unwrap** the option to treat the two cases. To do so there are various ways, here are 3 of them:

```rust
fn main() {
    let my_opt: Option<u32> = Some(42);

    // With match
    match my_opt {
        Some(val) => println!("{}", val),
        None => println!("None"),
    }
    // By unwrapping the value
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
        // the else bloc must interrupt
        return;
    };
    // ... val is in this scope !
    println!("{}", val);
}
```