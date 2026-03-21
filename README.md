# mito-reth

Mitosis execution client built on [reth](https://github.com/paradigmxyz/reth) v1.11.3. A thin overlay that extends the standard Ethereum reth node with a single Mitosis-specific behaviour: a one-time Multicall3 bytecode injection at the devnet hardfork block.

> Alpha software: `mito-reth` is currently in alpha and under active testing. Expect rough edges, incomplete validation, and breaking changes between releases.

## What it is

`mito-reth` is an upstream reth v1.11.3 Ethereum node with one addition:

**Multicall3 bytecode hook** — at block 786,000 on chain `124816` (Mitosis devnet), the canonical Multicall3 contract bytecode is written to `0xca11bde05977b3631167028862be2a173976ca11` as a pre-execution state change. This makes standard tooling that relies on that address work from genesis without a separate deployment transaction. On any other chain or block the hook is a no-op, so `mito-reth` can be used as a drop-in replacement for `reth` on vanilla Ethereum networks.

Everything else — EVM factory, precompiles, block assembly, RPC, P2P — is standard upstream reth v1.11.3.

## Workspace structure

```
Cargo.toml              Workspace root — shared deps, reth v1.11.3 pinned here
crates/
  primitives/           mito-reth-primitives — Multicall3 address, block number, chain ID, bytecode
  evm/                  mito-reth-evm — MitosisEvmConfig, executor wrapper, system call hook
bin/
  mito-reth/            mito-reth binary — CLI entry point (thin wrapper over reth CLI)
    src/main.rs           Wires MitosisExecutorBuilder into the reth node builder
```

## Building

Requires Rust ≥ 1.91 (effective MSRV — `alloy-consensus` 1.7.3 in `Cargo.lock` requires 1.91).

```bash
# debug
cargo build

# release
cargo build --release

# verify
./target/debug/mito-reth --help
```

The first build fetches reth v1.11.3 from git — expect several minutes.

## Install

Install the latest published binary:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/mitosis-org/mito-reth/main/scripts/install.sh | sh
```

Install a specific release tag:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/mitosis-org/mito-reth/main/scripts/install.sh | sh -s -- --version v0.1.0
```

The installer detects Linux/macOS/Windows and downloads the matching GitHub Release artifact into `/usr/local/bin` when writable, otherwise `~/.local/bin`. Published binaries currently target Linux `amd64`/`arm64`, macOS `arm64`, and Windows `amd64`.

Release builds can also be tested without pushing a tag by running the `Release` workflow manually from GitHub Actions. The manual path is dry-run only: it builds and uploads workflow artifacts, but does not publish a GitHub Release.

## Testing

```bash
cargo test
```

Tests currently live in the individual crates. The codebase is small; most correctness comes from the tight upstream dependency and the narrow scope of the overlay.

## Running

`mito-reth` accepts the same CLI flags as `reth`. Example:

```bash
mito-reth node \
  --chain <genesis.json> \
  --authrpc.jwtsecret <jwt.hex> \
  --authrpc.port 8551
```

The Engine API is exposed on `--authrpc.port` (default `8551`) over HTTP/IPC, identical to upstream reth.

## License

MIT OR Apache-2.0
