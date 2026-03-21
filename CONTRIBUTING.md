# Contributing to mito-reth

## Building

```bash
cargo build                    # debug
cargo build --release          # release
cargo build -p mito-reth-evm   # single crate
```

> First build fetches reth v1.11.3 from git and may take several minutes.

**MSRV:** Rust 1.91 (effective — `alloy-consensus` in `Cargo.lock` requires 1.91; workspace declares 1.88 but lock resolves higher).

## Testing

```bash
cargo test                          # all workspace crates
cargo test -p mito-reth-primitives
cargo test -p mito-reth-evm
```

## Linting

```bash
cargo check
RUSTFLAGS="-D warnings" cargo check   # zero warnings (matches CI)
cargo +nightly fmt --all              # format
cargo +nightly fmt --all --check      # check formatting
```

## Code style

- Edition 2024; workspace deps declared in root `Cargo.toml`, crates reference them with `dep.workspace = true`
- Formatting via `cargo +nightly fmt`
- Zero-warning policy enforced in CI (`RUSTFLAGS="-D warnings"`)
- **Thin wrapper pattern:** every Mitosis type wraps the corresponding upstream reth type and delegates all methods; the only override is `apply_pre_execution_changes`
- System-call hooks must be safe to call unconditionally — chain ID and block number guards belong inside the hook function, not at the call site
