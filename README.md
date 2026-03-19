# mito-reth

Mitosis execution client built on [reth](https://github.com/paradigmxyz/reth) v1.11.3. A thin overlay that extends the standard Ethereum reth node with a single Mitosis-specific behaviour: a one-time Multicall3 bytecode injection at the devnet hardfork block.

## What it is

`mi-reth` is an upstream reth v1.11.3 Ethereum node with one addition:

**Multicall3 bytecode hook** — at block 786,000 on chain `124816` (Mitosis devnet), the canonical Multicall3 contract bytecode is written to `0xca11bde05977b3631167028862be2a173976ca11` as a pre-execution state change. This makes standard tooling that relies on that address work from genesis without a separate deployment transaction. On any other chain or block the hook is a no-op, so `mi-reth` can be used as a drop-in replacement for `reth` on vanilla Ethereum networks.

Everything else — EVM factory, precompiles, block assembly, RPC, P2P — is standard upstream reth v1.11.3.

## Workspace structure

```
Cargo.toml              Workspace root — shared deps, reth v1.11.3 pinned here
crates/
  primitives/           mi-reth-primitives — Multicall3 address, block number, chain ID, bytecode
  evm/                  mi-reth-evm — MitosisEvmConfig, executor wrapper, system call hook
bin/
  mi-reth/              mi-reth binary — CLI entry point (thin wrapper over reth CLI)
    src/main.rs           Wires MitosisExecutorBuilder into the reth node builder
    src/ress.rs           Ress subprotocol wiring (disabled — see below)
```

## Building

Requires Rust ≥ 1.91 (effective MSRV — `alloy-consensus` 1.7.3 in `Cargo.lock` requires 1.91).

```bash
# debug
cargo build

# release
cargo build --release

# verify
./target/debug/mi-reth --help
```

The first build fetches reth v1.11.3 from git — expect several minutes.

## Testing

```bash
cargo test
```

Tests currently live in the individual crates. The codebase is small; most correctness comes from the tight upstream dependency and the narrow scope of the overlay.

## Running

`mi-reth` accepts the same CLI flags as `reth`. Example:

```bash
mi-reth node \
  --chain <genesis.json> \
  --authrpc.jwtsecret <jwt.hex> \
  --authrpc.port 8551
```

The Engine API is exposed on `--authrpc.port` (default `8551`) over HTTP/IPC, identical to upstream reth.

## Ress subprotocol (disabled)

The `ress` stateless-sync RLPx subprotocol wiring lives in `bin/mi-reth/src/ress.rs` but is not compiled by default. The `reth-ress-protocol` and `reth-ress-provider` crates from the Mitosis fork depend on reth internals that conflict with upstream v1.11.3 when mixed in the same Cargo workspace. Enabling `ress` requires porting those two crates to depend on upstream v1.11.3 directly. Track this work in a child issue of MIT-307.

## License

MIT OR Apache-2.0
