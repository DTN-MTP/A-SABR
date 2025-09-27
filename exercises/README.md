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
From this point, you should be able to compile and run the exercises. Cargo is the package manager and build system for Rust. It allows you to manage dependencies, build projects, and run separate examples. The exercises are provided as Cargo examples. You can compile and run a specific exercise using the following command:

```bash
cargo run --example <exercise-name>
```

Each exercise is contained in a folder that includes a <exercise-name>.rs file, which you can modify to complete the exercise. An exercise folder may also include:
- README.md to provides context or instructions for the exercise.
- Additional supporting files, such as data or configuration files.

## Rust Basics

Rust is a low-level language like C that provides high-level concepts like C++, but with guarantees of memory safety. The macro system is much more capable, and the policy is safety by default. For example struct members will be private by default, all variables will be immutable by default, etc.

The A-SABR library relies on polymorphic features that allow compile-time and runtime flexibility. This section will go through some important concepts and how they are implemented in Rust.



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

In the following example, we now want to store a list of concrete types implementing `MyTrait`. Templating aligns the templated type in memory, so as long as the types implementing `MyTrait` may have different sizes, only their addresses can be aligned. The Rust Vector (`Vec`) is a C/C++-like dynamic array that aligns the elements in memory and is therefore a templated type. If we want to store addresses (with `&` as in C) to concrete types implementing my trait, the `dyn` keyword will be required.

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
        // will panic, only one mutable borrow at a time
        // let mut illegal = heap_allocated.borrow_mut();
    } // borrowed leaves the scope
    let mut legal = heap_allocated.borrow_mut();
}
```

### Dynamic allocation and Traits

If the types are not in the stack but in the heap, the logic for polymorphism will also involve the `dyn` keyword, and the type `&dyn MyTrait` can be replaced by `Box<dyn MyTrait>`:

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
    let my_box_1: Box<TypeA> = Box::new(TypeA { member: 34 });
    let my_box_2:  Box<TypeB<TypeA>> = Box::new(TypeB {
        member1: 3.14,
        member2: TypeA { member: 42 },
    });

    // A list of boxes to structures implementing "MyTrait"
    let my_list: Vec<Box<dyn MyTrait>> = vec![my_box_1, my_box_2];
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

The `Result` type works the same way, but must be also templated by an error type and `Ok()` replaces `Some()` and `Err()` will be used instead of `None`.
