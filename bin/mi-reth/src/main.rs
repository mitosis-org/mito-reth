//! `mi-reth` — Mitosis execution node built on upstream reth v1.11.3.
//!
//! Upstream reth Ethereum node with one thin addition:
//!
//! **Multicall3 bytecode hook** — [`MitosisExecutorBuilder`] injects a
//! one-time state write at block 786 000 on `chain_id = 124816`.  On every
//! other chain or block the hook is a no-op; this binary can be used as a
//! drop-in replacement for `reth` on vanilla Ethereum networks.
//!
//! ## Remaining fork point: ress subprotocol
//!
//! The `ress` stateless-sync RLPx subprotocol wiring lives in `src/ress.rs`
//! but is NOT compiled by default.  The `reth-ress-protocol` and
//! `reth-ress-provider` crates from the Mitosis fork depend on fork-specific
//! reth networking internals that conflict with upstream v1.11.3 when mixed
//! in the same Cargo workspace.  Enabling ress requires porting those two
//! crates to depend on upstream reth v1.11.3 directly.  See `Cargo.toml`.

#![warn(unused_crate_dependencies)]

use mi_reth_evm::MitosisExecutorBuilder;
use reth_ethereum::{
    cli::Cli,
    node::{builder::NodeHandle, EthereumAddOns, EthereumNode},
};

fn main() -> eyre::Result<()> {
    Cli::parse_args().run(|builder, _args| async move {
        let NodeHandle { node: _node, node_exit_future } = builder
            // Replace the default executor with the Mitosis-aware one.
            // Everything else (EVM factory, precompiles, RPC, P2P) is
            // standard upstream reth v1.11.3.
            .with_types::<EthereumNode>()
            .with_components(EthereumNode::components().executor(MitosisExecutorBuilder::default()))
            .with_add_ons(EthereumAddOns::default())
            .launch()
            .await?;

        node_exit_future.await
    })
}
