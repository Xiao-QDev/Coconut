# Coconut Programming Language

Coconut is a statically typed systems programming language implemented in Rust with LLVM as its compilation backend. The language adopts Go-inspired syntax conventions and provides built-in JIT execution capabilities.

[![Rust](https://img.shields.io/badge/Rust-1.95%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![LLVM](https://img.shields.io/badge/LLVM-22.1.6-blue?logo=llvm)](https://llvm.org/)
[![License](https://img.shields.io/badge/License-Apache--2.0-green.svg)](LICENSE)
[![GitHub Stars](https://img.shields.io/github/stars/Xiao-QDev/Coconut?style=social)](https://github.com/Xiao-QDev/Coconut)

---

## Project Overview

Coconut aims to explore modern compiler construction while maintaining simplicity and performance. The compiler implements a complete compilation pipeline including lexical analysis, syntactic analysis, intermediate representation generation, and just-in-time execution.

## Language Features

- Statically typed with type inference capabilities
- Go-style variable declaration and assignment syntax
- Complete control flow structures including conditional branching and looping
- Modular package and import system
- LLVM-based native code generation
- Integrated JIT execution engine
- Minimal runtime overhead

---

## Getting Started

### Prerequisites

- Rust toolchain 1.95 or later
- LLVM 22.1.6 development libraries

### Building from Source

```bash
git clone https://github.com/Xiao-QDev/Coconut.git
cd Coconut
cargo build --release
```

### Example Program

Create a source file named `example.coconut`:

```coconut
package main

import pio

fn main() int {
    pio.println("Coconut Compiler Demonstration")
    
    var sum int = 0
    for var i int = 0; i < 10; i = i + 1 {
        sum = sum + i
    }
    
    pio.print("Computed sum: ")
    pio.printi(sum)
    pio.println("")

    return 0
}
```

Execute with JIT compiler:

```bash
Open your Coconut_Compiler
Execute: ./cococ example.coconut
```

---

## Language Specification

### Variable Declarations

```coconut
// Explicit type annotation
var counter int = 0

// Type inference via short declaration
message := "Hello, World"

// Reassignment
counter = counter + 1
```

### Function Definitions

```coconut
fn factorial(n int) int {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

fn main() int {
    var result int = factorial(5)
    return result
}
```

### Control Flow

```coconut
// Conditional branching
if value > threshold {
    pio.println("Above threshold")
} else if value == threshold {
    pio.println("At threshold")
} else {
    pio.println("Below threshold")
}

// Three-part for loop
for var i int = 0; i < bound; i = i + 1 {
    // loop body
}

// Condition-only loop
var index int = 0
for index < limit {
    index = index + 1
}

// Infinite loop with break
for {
    if condition_met {
        break
    }
}
```

---

## Standard Library

### pio Module

The standard print I/O module provides basic output functionality:

| Function | Description |
|----------|-------------|
| `pio.println(s)` | Output string followed by newline |
| `pio.print(s)` | Output string without trailing newline |
| `pio.printi(n)` | Output integer value |

---

## Implementation Status

### Completed Features

- [x] Lexical analyzer with full tokenization
- [x] Recursive descent parser with abstract syntax tree generation
- [x] LLVM intermediate representation code generation
- [x] JIT execution engine integration
- [x] Integer and string primitive types
- [x] Variable declaration and assignment
- [x] Function definition and invocation
- [x] If-else conditional branching
- [x] Multi-form for loop constructs
- [x] Break and continue statements
- [x] Package and import resolution
- [x] Basic standard library implementation

### Planned Features

- [ ] Floating-point and boolean primitive types
- [ ] Array and slice data structures
- [ ] Structure definitions and method implementations
- [ ] Pointer types and memory operations
- [ ] Enhanced error reporting with source location
- [ ] Comprehensive unit test coverage
- [ ] Compiler optimization passes
- [ ] Self-hosting compiler implementation
- [ ] Extended standard library
- [ ] Cross-compilation support

---

## Contributing

Contributions to the Coconut compiler project are welcome. Participants may submit issues for bug reports and feature requests, or open pull requests for code contributions.

---

## License

This project is licensed under the Apache License 2.0. See the LICENSE file for complete terms.

---

## Development Team

Developed and maintained by **Coconut-Dev-Team (CDT)**