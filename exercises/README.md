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

## Experimenting with the library

From this point, you should be able to compile and run the exercises. Cargo is the package manager and build system for Rust. It allows you to manage dependencies, build projects, and run separate examples. The exercises are provided as Cargo examples. You can compile and run a specific exercise using the following command:

```bash
cargo run --example <exercise-name>
```

Each exercise is contained in a folder that includes a <exercise-name>.rs file, which you can modify to complete the exercise. An exercise folder may also include:
- README.md to provides context or instructions for the exercise.
- Additional supporting files, such as data or configuration files.
