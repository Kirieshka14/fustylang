#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
MANIFEST="$ROOT/Cargo.toml"

case "${1:-}" in
  run)
    shift
    exec cargo run --manifest-path "$MANIFEST" -p fusty_cli -- run "$@"
    ;;
  demo)
    shift
    exec cargo run --manifest-path "$MANIFEST" -p fusty_cli -- "$@"
    ;;
  test)
    shift
    exec cargo test --manifest-path "$MANIFEST" "$@"
    ;;
  *)
    exec cargo run --manifest-path "$MANIFEST" -p fusty_cli -- "$@"
    ;;
esac
