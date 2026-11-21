# Contributing to FustyLang

Thank you for your interest in contributing to FustyLang!

## Development Setup

1. Ensure you have a recent stable Rust toolchain installed (`rustup` / `cargo`).
2. Clone the repository and run the workspace test suite:

```bash
cargo test --workspace
```

3. Run an example script using the CLI:

```bash
cargo run -p fusty_cli -- run examples/boss_fight.fy
```

## Guidelines

- Keep dependencies minimal (zero external crates in the core workspace).
- Add unit tests for any new lexer, parser, bytecode, or VM behavior.
- Run `cargo fmt --check` and `cargo test --workspace` before opening a pull request.
