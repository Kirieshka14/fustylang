# FustyLang (`Fusty` / `.fy`)

**FustyLang** is a fast, game-first, Rust-inspired scripting language implemented in pure Rust with **zero external dependencies**. It features built-in coroutines (`wait`, `yield`), first-class 2D and 3D vector primitives (`vec2`, `vec3`), an $O(1)$ per-frame bump arena allocator (`FrameArena`), and explicit safety-bypass scopes (`meow { ... }` / `meow fn`).

## Key Features

- **First-Class Game Vectors (`vec2`, `vec3`):** Native bytecode instructions for 2D/3D vector arithmetic, `.length()`, `.normalized()`, `.dot()`, `.cross()`, `.distance()`, and coordinate mutation (`v.x += 1.0`, `p.y = ...`).
- **Native Coroutines (`wait`, `yield`):** Pause execution mid-function for a duration (`wait 0.016;`) or yield back to the host loop without losing call-stack frames or local variables.
- **`meow` Safety Bypass Blocks:** Replaces `unsafe` with `meow { ... }` and `meow fn` scopes tracked at runtime by the VM's call-frame depth counter (`meow_depth`) to gate privileged host intrinsics.
- **Zero-GC Frame Arena & Step Budgets:** Deterministic $O(1)$ per-tick bump memory arena (`FrameArena`) and configurable instruction step budgets (`DEFAULT_STEP_BUDGET = 200_000`) to prevent infinite loops.
