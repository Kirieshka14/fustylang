# FustyLang (`Fusty` / `.fy`) — Tsundere & Native JIT Edition

**FustyLang** is a fast, game-first, Rust-inspired scripting language with a **tsundere compiler personality** and a **pure-Rust x86-64 native JIT compiler**, implemented with **zero external dependencies**. It features `nya` function declarations, `baka!(...)` panics, `meow { ... }` safety-bypass blocks, built-in coroutines (`wait`, `yield`), first-class 2D/3D vector primitives (`vec2`, `vec3`), and an $O(1)$ per-frame bump arena allocator (`FrameArena`).

> *"H-hmph! It's not like I JIT-compiled your hot `nya` functions into native x86-64 machine code just for you or anything... b-baka! >///<"*

## Key Features

- **Pure-Rust Native x86-64 Tiered JIT (`crates/fusty_jit`):** Zero-dependency machine-code assembler + W^X `mmap`/`mprotect` allocator + hot-call profiler. Automatically compiles hot `nya` functions (`i64` integer + `f64` SSE2 floating-point, loops, branches, and self-recursion) into native x86-64 machine code with seamless VM fallback for coroutines, vectors, and `baka!`.
- **Femboy / Kawaii Rust-Inspired Syntax:**
  - `nya` replaces `fn` for function declarations (`nya fib(n: i64) -> i64 { ... }`).
  - `a uwu b` is the **cuddle operator**: the midpoint of two numbers, `vec2`s or `vec3`s (`vec2(0.0, 0.0) uwu vec2(10.0, 20.0)` = `vec2(5, 10)`).
  - `baka!(...)` replaces `panic!(...)` for explicit runtime panics (`[BAKA! >///<]`).
  - `meow { ... }` / `meow nya ...` replaces `unsafe` with runtime `meow_depth` tracking.
  - `purr(...)` built-in cute logger alongside `log(...)`.
- **Tsundere Compiler Diagnostics:** Lexer, parser, bytecode compiler, JIT runtime, and VM all report mistakes in character (`"B-baka! You can't touch immutable variable 'x'! Add let mut if you really want to change it... >///<"`).
- **First-Class Game Vectors (`vec2`, `vec3`) & Coroutines (`wait`, `yield`):** Native bytecode instructions for vector math and zero-loss coroutine suspension.
- **Zero-GC Frame Arena & Step Budgets:** Deterministic $O(1)$ per-tick bump allocator (`FrameArena`) and configurable step budgets (`DEFAULT_STEP_BUDGET = 200_000`).

## Architecture & Crates

| Crate | Description |
| :--- | :--- |
| `crates/fusty_lexer` | Span-aware tokenizer with `nya`, `baka`, `meow`, `wait`, `yield`, `vec2`, `vec3`, nested `/* ... */` comments, and tsundere lexer errors |
| `crates/fusty_parser` | Recursive-descent & Pratt precedence-climbing AST parser supporting `nya`, `baka!(...)`, `struct`, `enum`, `impl`, `match`, `if/else`, and `while` |
| `crates/fusty_bytecode` | 3-pass bytecode compiler with forward function resolution, function bytecode bounds (`entry_ip..end_ip`), immutability checks, and `Opcode::Baka` |
| `crates/fusty_jit` | Zero-dependency pure-Rust x86-64 JIT compiler: W^X `mmap` allocator, GPR + SSE2 assembler, bytecode analyzer, and tiered hot-call `JitEngine` |
| `crates/fusty_vm` | Stack-based VM with `CallFrame` management, Tiered JIT dispatch, `Host` FFI trait, `FrameArena`, and coroutine suspension |
| `crates/fusty_cli` | Standalone CLI runner & benchmarker (`fusty run`, `fusty bench`, `--jit`, `--no-jit`, `--jit-stats`) |

## Quick Start

```bash
# Run the built-in FustyLang ~nya combat demo
cargo run -p fusty_cli

# Run a FustyLang (.fy) script with JIT stats
cargo run -p fusty_cli -- run examples/fibonacci.fy --jit-stats

# Benchmark Bytecode VM vs Native x86-64 JIT
cargo run -p fusty_cli -- bench examples/jit_benchmark.fy

# Run with Eager JIT or pure VM interpreter
cargo run -p fusty_cli -- run examples/tsundere_baka.fy --jit
cargo run -p fusty_cli -- run examples/tsundere_baka.fy --no-jit

# Run all unit & parity tests across every crate
cargo test --workspace
```

## Example Script (`.fy`)

```rust
nya fib(n: i64) -> i64 {
    if n <= 1 {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

nya check_hp(hp: i64) -> i64 {
    if hp <= 0 {
        baka!("Hmph! Zero HP — don't expect me to revive you! >///<");
    }
    return hp;
}

let mut pos = vec2(100.0, 50.0);
let target = vec2(300.0, 200.0);
let dir = (target - pos).normalized();
pos += dir * 250.0 * 0.016;

wait 0.25;

meow {
    let fast_val = fib(20);
    purr("JIT-accelerated fib(20) =", fast_val);
}

let hp = check_hp(100);
purr("pos =", pos, "hp =", hp);
```

## License

This project is licensed under the [MIT License](LICENSE).
