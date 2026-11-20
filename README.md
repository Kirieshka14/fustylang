# FustyLang (`Fusty` / `.fy`)

**FustyLang** is a fast, game-first, Rust-inspired scripting language implemented in pure Rust with **zero external dependencies**. It features built-in coroutines (`wait`, `yield`), first-class 2D and 3D vector primitives (`vec2`, `vec3`), an $O(1)$ per-frame bump arena allocator (`FrameArena`), and explicit safety-bypass scopes (`meow { ... }` / `meow fn`).

## Key Features

- **First-Class Game Vectors (`vec2`, `vec3`):** Native bytecode instructions for 2D/3D vector arithmetic, `.length()`, `.normalized()`, `.dot()`, `.cross()`, `.distance()`, and coordinate mutation (`v.x += 1.0`, `p.y = ...`).
- **Native Coroutines (`wait`, `yield`):** Pause execution mid-function for a duration (`wait 0.016;`) or yield back to the host loop without losing call-stack frames or local variables.
- **`meow` Safety Bypass Blocks:** Replaces `unsafe` with `meow { ... }` and `meow fn` scopes tracked at runtime by the VM's call-frame depth counter (`meow_depth`) to gate privileged host intrinsics.
- **Zero-GC Frame Arena & Step Budgets:** Deterministic $O(1)$ per-tick bump memory arena (`FrameArena`) and configurable instruction step budgets (`DEFAULT_STEP_BUDGET = 200_000`) to prevent infinite loops.

## Architecture & Crates

| Crate | Description |
| :--- | :--- |
| `crates/fusty_lexer` | Span-aware tokenizer with nested multi-line comments (`/* ... */`), numeric/string literals, and `meow` / `wait` / `yield` / `vec2` / `vec3` keywords |
| `crates/fusty_parser` | Recursive-descent and Pratt precedence-climbing AST parser supporting functions, `struct`, `enum`, `impl`, `match`, `if/else`, and `while` loops |
| `crates/fusty_bytecode` | 3-pass bytecode compiler with forward function resolution, immutability checks (`let` vs `let mut`), loop stack unwinding, and vector intrinsics |
| `crates/fusty_vm` | Stack-based bytecode Virtual Machine with `CallFrame` management, `Host` FFI trait, `FrameArena`, and coroutine suspension (`VmResult::Waiting` / `Yielded`) |
| `crates/fusty_cli` | Standalone CLI compiler and runner (`fusty run <script.fy>`) |

## Quick Start

```bash
# Run the built-in FustyLang combat & coroutine demo
cargo run -p fusty_cli

# Run a FustyLang (.fy) script file
cargo run -p fusty_cli -- run examples/boss_fight.fy

# Run all unit tests across lexer, parser, bytecode compiler, and VM
cargo test
```

## Example Script (`.fy`)

```rust
let spin_speed = 1.6;
let bob_amp = 0.25;

fn compute_offset(t: f64) -> vec3 {
    let mut p = vec3(0.0, 0.0, 0.0);
    p.y = abs(sin(t * 2.0)) * bob_amp;
    return p;
}

let mut pos = vec2(100.0, 50.0);
let target = vec2(300.0, 200.0);
let dir = (target - pos).normalized();
pos += dir * 250.0 * 0.016;

wait 0.25;

meow {
    let fast_val = 1337;
    log("meow fast_val =", fast_val);
}

let offset3 = compute_offset(1.5);
log("pos =", pos, "offset3 =", offset3);
```
