# Changelog

All notable changes to FustyLang will be documented in this file.

## [0.2.1] - 2026-11-23

### Added
- `uwu` cuddle operator: `a uwu b` is the midpoint of two values (numbers always give a float, `vec2`/`vec3` cuddle component-wise). Binds tighter than comparisons, looser than `+`/`-`. VM-only (JIT falls back cleanly). Mismatched types get a tsundere error.

## [0.2.0] - 2026-11-23

### Added
- **Pure-Rust Native x86-64 Tiered JIT (`fusty_jit`)** with zero external dependencies:
  - W^X executable memory allocator via Linux `mmap`/`mprotect`/`munmap`.
  - Custom x86-64 machine code assembler for GPR integer ops, SSE2 `f64` math, labels, branches, loops, and self-recursion.
  - Bytecode eligibility analyzer, type specialization (`i64` / `f64`), and hot-call `JitEngine` with automatic VM fallback.
- **`baka!(...)` panic expression & `Opcode::Baka`** replacing `panic!` with tsundere runtime diagnostics (`[BAKA! >///<]`).
- **Tsundere compiler personality** across lexer, parser, bytecode compiler, JIT runtime, VM, and CLI.
- **`purr(...)` built-in logger** alongside `log(...)` in `fusty_cli`.
- **CLI JIT controls & benchmarker:** `--jit`, `--no-jit`, `--jit-stats`, and `fusty bench <file.fy>`.
- New examples (`examples/jit_benchmark.fy`, `examples/tsundere_baka.fy`) and comprehensive JIT-vs-VM parity tests.

### Changed
- **Breaking:** Replaced `fn` keyword with `nya` across lexer, parser, tests, examples, and documentation (`meow fn` is now `meow nya`).
- Extended `FunctionMeta` with `end_ip` bytecode bounds for exact JIT compilation ranges.

## [0.1.0] - 2026-11-21

### Added
- Zero-dependency Rust workspace (`fusty_lexer`, `fusty_parser`, `fusty_bytecode`, `fusty_vm`, `fusty_cli`).
- Span-aware lexer with nested multi-line comments, `vec2`/`vec3`, `wait`, `yield`, and `meow` keywords.
- Recursive-descent and Pratt parser for functions, structs, enums, `impl`, `match`, `if/else`, and `while`.
- 3-pass bytecode compiler with immutability enforcement and vector intrinsics.
- Stack-based VM with `FrameArena`, step budgets, and coroutine suspension.
- Standalone CLI (`fusty run <script.fy>`) and example scripts.
- MIT License.
