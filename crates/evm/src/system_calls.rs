//! Mitosis system calls — one-time Multicall3 bytecode replacement.
//!
//! This is the **only** execution-layer behaviour that diverges from vanilla
//! Ethereum in the Mitosis network.  At block 786 000 on chain_id 124816 the
//! well-known `0xca11...` address receives canonical Multicall3 bytecode so
//! that tooling relying on that address works from genesis without a separate
//! deployment transaction.

use alloy_evm::{Evm, block::BlockExecutionError};
use alloy_primitives::{U256, keccak256};
use mito_reth_primitives::{
    MULTICALL3_ADDRESS, MULTICALL3_HARDFORK_CHAIN_ID, MULTICALL3_REPLACEMENT_BLOCK,
    get_multicall3_bytecode,
};
use revm::context_interface::block::Block;
use revm::{
    DatabaseCommit,
    database::Database,
    state::{Account, AccountInfo, AccountStatus, Bytecode, EvmState},
};

/// Write Multicall3 bytecode directly into EVM state.
///
/// The function is gated by chain-id and block number so it is safe to call
/// unconditionally on every block; it becomes a no-op everywhere except the
/// one targeted block on the Mitosis devnet.
#[inline]
pub fn deploy_multicall3_contract<Halt, E>(evm: &mut E) -> Result<(), BlockExecutionError>
where
    E: Evm<HaltReason = Halt> + ?Sized,
    E::DB: Database + DatabaseCommit,
{
    if evm.chain_id() != MULTICALL3_HARDFORK_CHAIN_ID {
        return Ok(());
    }
    if evm.block().number().saturating_to::<u64>() != MULTICALL3_REPLACEMENT_BLOCK {
        return Ok(());
    }

    // Load the existing account into the REVM cache before mutating it —
    // skipping this step causes cache-consistency panics inside revm.
    let existing = evm
        .db_mut()
        .basic(MULTICALL3_ADDRESS)
        .map_err(|_| BlockExecutionError::msg("failed to load Multicall3 account into cache"))?;

    let bytecode = get_multicall3_bytecode();
    let code_hash = keccak256(&bytecode);
    let balance = existing.map(|a| a.balance).unwrap_or(U256::ZERO);

    let account_info = AccountInfo {
        balance,
        nonce: 1, // non-zero nonce marks this as a deployed contract
        code_hash,
        code: Some(Bytecode::new_raw(bytecode)),
        account_id: None,
    };

    let account = Account {
        info: account_info.clone(),
        original_info: Box::new(account_info),
        transaction_id: 0,
        storage: Default::default(),
        status: AccountStatus::Touched | AccountStatus::Created,
    };

    let mut state = EvmState::default();
    state.insert(MULTICALL3_ADDRESS, account);
    evm.db_mut().commit(state);

    Ok(())
}

/// Called from [`MitosisBlockExecutor::apply_pre_execution_changes`].
#[inline]
pub fn apply_multicall3_deployment<Halt, E>(evm: &mut E) -> Result<(), BlockExecutionError>
where
    E: Evm<HaltReason = Halt> + ?Sized,
    E::DB: Database + DatabaseCommit,
{
    deploy_multicall3_contract(evm)
}
