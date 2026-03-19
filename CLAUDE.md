# mito-reth

Mitosis execution client — upstream reth v1.11.3 with a thin Multicall3 bytecode hook. Binary: `mi-reth`.

## Build

```bash
cargo build                    # debug
cargo build --release          # release
cargo build -p mi-reth-evm     # single crate
```

MSRV: Rust 1.91 (effective — alloy-consensus 1.7.3 in `Cargo.lock` requires 1.91; workspace declares 1.88 but lock resolves higher). No `rust-toolchain.toml`.

First build fetches reth v1.11.3 from git — takes several minutes.

## Test

```bash
cargo test                     # all workspace crates
cargo test -p mi-reth-primitives
cargo test -p mi-reth-evm
```

## Check / Lint

```bash
cargo check
RUSTFLAGS="-D warnings" cargo check   # zero warnings (matches CI intent)
cargo +nightly fmt --all              # format
cargo +nightly fmt --all --check      # check formatting
```

## Crate layout

| Path | Crate | Purpose |
|------|-------|---------|
| `crates/primitives/` | `mi-reth-primitives` | Constants: `MULTICALL3_ADDRESS`, `MULTICALL3_REPLACEMENT_BLOCK` (786,000), `MULTICALL3_HARDFORK_CHAIN_ID` (124816). Bytecode accessor `get_multicall3_bytecode()`. |
| `crates/evm/` | `mi-reth-evm` | `MitosisEvmConfig` wraps `EthEvmConfig`. `MitosisBlockExecutorFactory` / `MitosisBlockExecutor` wrap upstream executor. `apply_multicall3_deployment` fires the hook. `MitosisExecutorBuilder` is the node-builder adapter. |
| `bin/mi-reth/` | `mi-reth` | Binary. `main.rs` wires `MitosisExecutorBuilder` into the reth node builder. |

## Key files

| File | What it does |
|------|-------------|
| `Cargo.toml` | Workspace root. Pins reth v1.11.3 and all alloy/revm versions. |
| `crates/primitives/src/lib.rs` | Multicall3 constants and `get_multicall3_bytecode()`. |
| `crates/evm/src/system_calls.rs` | `deploy_multicall3_contract` — the actual state write, gated by chain ID and block number. |
| `crates/evm/src/executor.rs` | `MitosisBlockExecutor` — calls `apply_pre_execution_changes` then fires the hook. |
| `crates/evm/src/config.rs` | `MitosisEvmConfig` — `ConfigureEvm` impl delegating to `EthEvmConfig`. |
| `crates/evm/src/lib.rs` | Public re-exports and `MitosisExecutorBuilder` impl. |
| `bin/mi-reth/src/main.rs` | CLI entry point using reth's `Cli::parse_args()`. |

## Code style

- Edition 2024, workspace deps in root `Cargo.toml`, crates use `dep.workspace = true`
- `cargo +nightly fmt` for formatting
- Zero-warning policy: `RUSTFLAGS="-D warnings"`
- Thin wrapper pattern: every Mitosis type wraps an upstream type and delegates all methods; the only override is `apply_pre_execution_changes`
- System-call hook must be safe to call unconditionally — chain ID and block number guards are inside the function, not at the call site

## Dependency versions (from workspace)

| Dep | Version |
|-----|---------|
| reth | v1.11.3 (git, paradigmxyz/reth) |
| alloy-evm | 0.27.2 |
| alloy-primitives | 1.5.6 |
| alloy-consensus | 1.6.3 |
| alloy-eips | 1.6.3 |
| revm | 34.0.0 |
