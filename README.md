# FustyLang

**FustyLang** (`.fy`) is a fast, game-first scripting language with Rust-inspired syntax and a native x86-64 JIT compiler. Written in Rust with zero external dependencies.

- Rust-like syntax with a few kawaii keywords
- Tiered JIT: hot functions are compiled to native machine code
- Built-in `vec2` / `vec3` and coroutines (`wait`, `yield`)
- Per-frame arena allocator (`FrameArena`) instead of a GC

## Table of Contents

- [Quick Start](#quick-start)
- [Syntax Tutorial](#syntax-tutorial)
- [What's New](#whats-new)
- [Architecture](#architecture)
- [Roadmap](#roadmap)
- [License](#license)

## Quick Start

```bash
# Run the built-in demo
cargo run -p fusty_cli

# Run a script (with JIT stats)
cargo run -p fusty_cli -- run examples/fibonacci.fy --jit-stats

# Benchmark: VM vs JIT
cargo run -p fusty_cli -- bench examples/jit_benchmark.fy

# Force eager JIT / pure VM interpreter
cargo run -p fusty_cli -- run examples/tsundere_baka.fy --jit
cargo run -p fusty_cli -- run examples/tsundere_baka.fy --no-jit

# Run tests
cargo test --workspace
```

## Syntax Tutorial

### Variables

```rust
let x = 10;          // immutable
let mut y = 5.0;     // mutable
y += 1.0;
```

### Functions: `nya`

`nya` replaces `fn`.

```rust
nya fib(n: i64) -> i64 {
    if n <= 1 { return n; }
    return fib(n - 1) + fib(n - 2);
}
```

### Panics: `baka!`

`baka!` replaces `panic!`.

```rust
baka!("something went wrong");
```

### Unsafe blocks: `meow`

`meow` replaces `unsafe`.

```rust
meow {
    purr("inside a meow block");
}
```

### The `uwu` operator

`a uwu b` returns the midpoint of two values. It works on numbers (the result is always a float), `vec2` and `vec3`. It binds tighter than comparisons, looser than `+`/`-`, and is left-associative.

```rust
1 uwu 2;                                   // 1.5
vec2(0.0, 0.0) uwu vec2(10.0, 20.0);       // vec2(5, 10)
```

### Vectors

```rust
let mut pos = vec2(100.0, 50.0);
let dir = (vec2(300.0, 200.0) - pos).normalized();
pos += dir * 250.0 * 0.016;
```

### Coroutines

```rust
wait 0.25;   // pause for 0.25 seconds
yield;       // give control back until the next frame
```

### Control flow

`if` / `else`, `while`, `for`, `match`, `return`, `break`, `continue`, plus `struct`, `enum` and `impl`.

### Built-in functions

| Function | Description |
| :--- | :--- |
| `purr(...)` | print values (cute logger) |
| `log(...)` | print values |

### Full example

```rust
nya check_hp(hp: i64) -> i64 {
    if hp <= 0 {
        baka!("out of HP");
    }
    return hp;
}

let mut pos = vec2(100.0, 50.0);
wait 0.25;

meow {
    purr("pos =", pos, "hp =", check_hp(100));
}
```

More examples are in the [`examples/`](examples) folder.

## What's New

### 0.2.1

- `uwu` operator: the midpoint of numbers, `vec2` and `vec3`.

### 0.2.0

- Native x86-64 tiered JIT with zero dependencies; `--jit`, `--no-jit`, `--jit-stats` flags and the `bench` command.
- `fn` is now `nya`, `panic!` is now `baka!`.
- Tsundere-style error messages.

See [CHANGELOG.md](CHANGELOG.md) for the full history.

## Architecture

| Crate | Purpose |
| :--- | :--- |
| `crates/fusty_lexer` | Span-aware tokenizer, nested `/* */` comments |
| `crates/fusty_parser` | AST parser (recursive descent + precedence climbing) |
| `crates/fusty_bytecode` | Bytecode compiler, immutability checks |
| `crates/fusty_jit` | x86-64 JIT: W^X allocator, GPR + SSE2 assembler, bytecode analyzer, tiered engine |
| `crates/fusty_vm` | Stack-based VM, JIT dispatch, `Host` FFI, `FrameArena`, coroutines |
| `crates/fusty_cli` | CLI: `run`, `bench`, JIT flags |

## Roadmap

The main focus is improving and extending the JIT.

- [ ] `vec2` / `vec3` support in the JIT (SSE registers)
- [ ] `uwu` and other new operators in the JIT
- [ ] Calls between different JIT-compiled functions (not only self-recursion)
- [ ] Inlining of small functions
- [ ] Optimizations: constant folding, dead code elimination, register allocation
- [ ] Hot loop compilation (OSR)
- [ ] `aarch64` backend
- [ ] Better JIT diagnostics (machine code dump)

## License

[MIT](LICENSE)
