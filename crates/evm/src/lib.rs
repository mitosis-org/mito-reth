//! Mitosis EVM customisation layer.
//!
//! This crate provides the thin wrapper types needed to run an upstream reth
//! v1.11.3 Ethereum node with the Mitosis-specific Multicall3 pre-execution
//! hook.
//!
//! ## Downstream surface
//!
//! | Type | Purpose |
//! |------|---------|
//! | [`MitosisEvmConfig`]            | Replaces `EthEvmConfig` in the node builder |
//! | [`MitosisExecutorBuilder`]      | Adapter for the reth node-builder API |
//! | [`MitosisBlockExecutorFactory`] | Wraps the upstream factory; injects the hook |
//! | [`MitosisBlockExecutor`]        | Wraps the upstream executor; fires the hook |
//!
//! Everything else — EVM factory, precompiles, block assembler, RPC, P2P — is
//! standard upstream reth.

mod config;
mod executor;
mod system_calls;

pub use config::MitosisEvmConfig;
pub use executor::{MitosisBlockExecutor, MitosisBlockExecutorFactory};
pub use system_calls::{apply_multicall3_deployment, deploy_multicall3_contract};

use reth_chainspec::ChainSpec;
use reth_ethereum::node::{
    api::{FullNodeTypes, NodeTypes},
    builder::{components::ExecutorBuilder, BuilderContext},
};
use reth_ethereum_primitives::EthPrimitives;

/// Node-builder executor component that installs [`MitosisEvmConfig`].
///
/// Usage in the node builder:
/// ```rust,ignore
/// builder
///     .with_types::<EthereumNode>()
///     .with_components(EthereumNode::components().executor(MitosisExecutorBuilder::default()))
///     .with_add_ons(EthereumAddOns::default())
///     .launch()
///     .await?;
/// ```
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct MitosisExecutorBuilder;

impl<Node> ExecutorBuilder<Node> for MitosisExecutorBuilder
where
    Node: FullNodeTypes<Types: NodeTypes<ChainSpec = ChainSpec, Primitives = EthPrimitives>>,
{
    type EVM = MitosisEvmConfig;

    async fn build_evm(self, ctx: &BuilderContext<Node>) -> eyre::Result<Self::EVM> {
        Ok(MitosisEvmConfig::new(ctx.chain_spec()))
    }
}
