# Changelog

All notable changes to FustyLang will be documented in this file.

## [0.1.0] - 2026-11-21

### Added
- Zero-dependency Rust workspace (`fusty_lexer`, `fusty_parser`, `fusty_bytecode`, `fusty_vm`, `fusty_cli`).
- Span-aware lexer with nested multi-line comments, `vec2`/`vec3`, `wait`, `yield`, and `meow` keywords.
- Recursive-descent and Pratt parser for functions, structs, enums, `impl`, `match`, `if/else`, and `while`.
- 3-pass bytecode compiler with immutability enforcement and vector intrinsics.
- Stack-based VM with `FrameArena`, step budgets, and coroutine suspension.
- Standalone CLI (`fusty run <script.fy>`) and example scripts.
- MIT License.
