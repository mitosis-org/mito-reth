//! [`MitosisEvmConfig`] — upstream [`EthEvmConfig`] wrapped with the Mitosis
//! block-executor factory so that the Multicall3 pre-execution hook fires on
//! every block.

use std::sync::Arc;

use alloy_consensus::Header;
use alloy_evm::{eth::EthBlockExecutionCtx, EvmEnv};
use reth_chainspec::ChainSpec;
use reth_ethereum::evm::{EthBlockAssembler, EthEvmConfig};
use reth_ethereum_primitives::{Block, EthPrimitives};
use reth_evm::{ConfigureEvm, NextBlockEnvAttributes};
use reth_primitives_traits::{SealedBlock, SealedHeader};
use revm::primitives::hardfork::SpecId;

use crate::MitosisBlockExecutorFactory;

/// Mitosis EVM configuration.
///
/// Wraps the upstream [`EthEvmConfig`] and replaces its
/// `BlockExecutorFactory` with [`MitosisBlockExecutorFactory`], which injects
/// the Multicall3 bytecode replacement into every block's pre-execution phase.
///
/// All other behaviour (EVM factory, precompiles, block assembler, env
/// helpers) is delegated unchanged to the inner config.
#[derive(Debug, Clone)]
pub struct MitosisEvmConfig {
    inner: EthEvmConfig,
    block_executor_factory:
        MitosisBlockExecutorFactory<<EthEvmConfig as ConfigureEvm>::BlockExecutorFactory>,
}

impl MitosisEvmConfig {
    /// Creates a new [`MitosisEvmConfig`] for the given chain spec.
    pub fn new(chain_spec: Arc<ChainSpec>) -> Self {
        let inner = EthEvmConfig::new(chain_spec);
        let block_executor_factory =
            MitosisBlockExecutorFactory::new(inner.block_executor_factory().clone());
        Self { inner, block_executor_factory }
    }
}

impl ConfigureEvm for MitosisEvmConfig {
    type Primitives = EthPrimitives;
    type Error = <EthEvmConfig as ConfigureEvm>::Error;
    type NextBlockEnvCtx = NextBlockEnvAttributes;
    type BlockExecutorFactory =
        MitosisBlockExecutorFactory<<EthEvmConfig as ConfigureEvm>::BlockExecutorFactory>;
    type BlockAssembler = EthBlockAssembler<ChainSpec>;

    fn block_executor_factory(&self) -> &Self::BlockExecutorFactory {
        &self.block_executor_factory
    }

    fn block_assembler(&self) -> &Self::BlockAssembler {
        &self.inner.block_assembler
    }

    fn evm_env(&self, header: &Header) -> Result<EvmEnv<SpecId>, Self::Error> {
        self.inner.evm_env(header)
    }

    fn next_evm_env(
        &self,
        parent: &Header,
        attributes: &NextBlockEnvAttributes,
    ) -> Result<EvmEnv, Self::Error> {
        self.inner.next_evm_env(parent, attributes)
    }

    fn context_for_block<'a>(
        &self,
        block: &'a SealedBlock<Block>,
    ) -> Result<EthBlockExecutionCtx<'a>, Self::Error> {
        self.inner.context_for_block(block)
    }

    fn context_for_next_block(
        &self,
        parent: &SealedHeader<Header>,
        attributes: NextBlockEnvAttributes,
    ) -> Result<EthBlockExecutionCtx<'_>, Self::Error> {
        self.inner.context_for_next_block(parent, attributes)
    }
}
