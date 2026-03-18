//! Mitosis block executor wrapper.
//!
//! [`MitosisBlockExecutor`] delegates all standard execution to the upstream
//! reth Ethereum executor and adds only the Multicall3 pre-execution hook in
//! [`apply_pre_execution_changes`](BlockExecutor::apply_pre_execution_changes).
//!
//! # Delegation is safe
//!
//! [`ExecutableTx<E>`] is a blanket impl over
//! `ExecutableTxParts<E::Evm::Tx, E::Transaction>` — it only cares about two
//! associated types.  Since `MitosisBlockExecutor<Inner>` has the same `Evm`
//! and `Transaction` types as `Inner`, any `tx: impl ExecutableTx<MitosisBlockExecutor<Inner>>`
//! is automatically `impl ExecutableTx<Inner>`, so all delegation calls compile.

use alloy_evm::{
    Evm, EvmFactory,
    block::{
        BlockExecutionError, BlockExecutionResult, BlockExecutor, BlockExecutorFactory,
        BlockExecutorFor, ExecutableTx, OnStateHook,
    },
};
use revm::{Database, database::State, inspector::Inspector};

use crate::system_calls::apply_multicall3_deployment;

// ---------------------------------------------------------------------------
// MitosisBlockExecutor
// ---------------------------------------------------------------------------

/// Wrapper that fires the Multicall3 pre-execution hook and delegates
/// everything else to the upstream Ethereum block executor.
#[derive(Debug)]
pub struct MitosisBlockExecutor<Inner> {
    inner: Inner,
}

impl<Inner> MitosisBlockExecutor<Inner> {
    /// Wraps an existing block executor.
    pub fn new(inner: Inner) -> Self {
        Self { inner }
    }
}

impl<Inner> BlockExecutor for MitosisBlockExecutor<Inner>
where
    Inner: BlockExecutor,
    <Inner::Evm as Evm>::DB: Database + revm::DatabaseCommit,
{
    type Transaction = Inner::Transaction;
    type Receipt = Inner::Receipt;
    type Evm = Inner::Evm;
    type Result = Inner::Result;

    // --- Mitosis hook -------------------------------------------------------

    /// Runs the upstream pre-execution changes then applies the one-time
    /// Multicall3 bytecode replacement if this is the targeted block.
    fn apply_pre_execution_changes(&mut self) -> Result<(), BlockExecutionError> {
        self.inner.apply_pre_execution_changes()?;
        apply_multicall3_deployment(self.inner.evm_mut())?;
        Ok(())
    }

    // --- Pure delegation: required abstract methods -------------------------

    fn execute_transaction_without_commit(
        &mut self,
        tx: impl ExecutableTx<Self>,
    ) -> Result<Self::Result, BlockExecutionError> {
        self.inner.execute_transaction_without_commit(tx)
    }

    fn commit_transaction(&mut self, output: Self::Result) -> Result<u64, BlockExecutionError> {
        self.inner.commit_transaction(output)
    }

    fn finish(
        self,
    ) -> Result<(Self::Evm, BlockExecutionResult<Self::Receipt>), BlockExecutionError> {
        self.inner.finish()
    }

    fn apply_post_execution_changes(
        self,
    ) -> Result<BlockExecutionResult<Self::Receipt>, BlockExecutionError> {
        self.inner.apply_post_execution_changes()
    }

    fn set_state_hook(&mut self, hook: Option<Box<dyn OnStateHook>>) {
        self.inner.set_state_hook(hook)
    }

    fn evm_mut(&mut self) -> &mut Self::Evm {
        self.inner.evm_mut()
    }

    fn evm(&self) -> &Self::Evm {
        self.inner.evm()
    }

    fn receipts(&self) -> &[Self::Receipt] {
        self.inner.receipts()
    }
}

// ---------------------------------------------------------------------------
// MitosisBlockExecutorFactory
// ---------------------------------------------------------------------------

/// Wraps an upstream [`BlockExecutorFactory`] and produces
/// [`MitosisBlockExecutor`] instances.
#[derive(Debug, Clone)]
pub struct MitosisBlockExecutorFactory<Inner> {
    inner: Inner,
}

impl<Inner> MitosisBlockExecutorFactory<Inner> {
    /// Creates a new factory wrapping an existing upstream factory.
    pub const fn new(inner: Inner) -> Self {
        Self { inner }
    }
}

impl<Inner> BlockExecutorFactory for MitosisBlockExecutorFactory<Inner>
where
    Inner: BlockExecutorFactory,
    Self: 'static,
{
    type EvmFactory = Inner::EvmFactory;
    type ExecutionCtx<'a> = Inner::ExecutionCtx<'a>;
    type Transaction = Inner::Transaction;
    type Receipt = Inner::Receipt;

    fn evm_factory(&self) -> &Self::EvmFactory {
        self.inner.evm_factory()
    }

    fn create_executor<'a, DB, I>(
        &'a self,
        evm: <Self::EvmFactory as EvmFactory>::Evm<&'a mut State<DB>, I>,
        ctx: Self::ExecutionCtx<'a>,
    ) -> impl BlockExecutorFor<'a, Self, DB, I>
    where
        DB: Database + std::fmt::Debug + 'a,
        I: Inspector<<Self::EvmFactory as EvmFactory>::Context<&'a mut State<DB>>> + 'a,
    {
        MitosisBlockExecutor::new(self.inner.create_executor(evm, ctx))
    }
}
