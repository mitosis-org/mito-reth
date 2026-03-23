//! Integration test: `--storage.v2` must correctly return logs via `eth_getLogs`
//! after a block is committed through the Engine API.
//!
//! # Why this test exists
//!
//! The Mitosis consensus layer (mitosisd / octane evmengine) drives execution by:
//!
//!   1. `engine_newPayloadV4`       — submit an EVM block
//!   2. `engine_forkchoiceUpdatedV3` — make it canonical
//!   3. `eth_getLogs(blockHash)`     — fetch EVM events that update Cosmos state
//!
//! Step 3 uses `retryForever` that only retries on RPC *errors*, not on an
//! empty result.  If `--storage.v2` causes `eth_getLogs` to silently return `[]`
//! for a just-committed block, the evmvalidator/evmgov Cosmos modules never
//! receive the events, the Cosmos multistore root (AppHash) diverges, and the
//! node is stuck.
//!
//! This test exercises the exact same path and asserts that logs are present.

#![cfg(all(feature = "rocksdb", unix))]

use alloy_consensus::BlockHeader;
use alloy_network::{Ethereum, EthereumWallet, TransactionBuilder, eip2718::Encodable2718};
use alloy_primitives::{Address, Bytes, TxKind, B256, U256};
use alloy_rpc_types_eth::{Log, TransactionInput, TransactionReceipt, TransactionRequest};
use alloy_signer_local::PrivateKeySigner;
use eyre::Result;
use jsonrpsee::core::client::ClientT;
use reth_chainspec::{ChainSpec, ChainSpecBuilder, MAINNET};
use reth_e2e_test_utils::{wallet::Wallet, E2ETestSetupBuilder};
use reth_node_ethereum::EthereumNode;
use reth_payload_builder::EthPayloadBuilderAttributes;
use std::{sync::Arc, time::Duration};

// ── Constants ─────────────────────────────────────────────────────────────────

/// keccak256("Transfer(address,address,uint256)") — the topic emitted by our
/// test contract.
const TRANSFER_TOPIC: B256 = B256::new([
    0xdd, 0xf2, 0x52, 0xad, 0x1b, 0xe2, 0xc8, 0x9b, 0x69, 0xc2, 0xb0, 0x68, 0xfc, 0x37, 0x8d,
    0xaa, 0x95, 0x2b, 0xa7, 0xf1, 0x63, 0xc4, 0xa1, 0x16, 0x28, 0xf5, 0x5a, 0x4d, 0xf5, 0x23,
    0xb3, 0xef,
]);

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Minimal EVM init-code (constructor) that emits one `LOG1` and then stops.
///
/// ```text
/// PUSH32 <TRANSFER_TOPIC>   // push topic
/// PUSH1  0x00               // size  = 0 bytes of memory data
/// PUSH1  0x00               // offset = 0
/// LOG1                      // emit the log
/// STOP
/// ```
/// The deployed contract is empty (constructor returns nothing), which is fine.
fn log_emitter_init_code() -> Bytes {
    let mut code: Vec<u8> = Vec::with_capacity(39);
    code.push(0x7f); // PUSH32
    code.extend_from_slice(TRANSFER_TOPIC.as_slice());
    code.extend_from_slice(&[0x60, 0x00]); // PUSH1 0  (size)
    code.extend_from_slice(&[0x60, 0x00]); // PUSH1 0  (offset)
    code.push(0xa1); // LOG1
    code.push(0x00); // STOP
    Bytes::from(code)
}

/// Build and sign a contract-creation (CREATE) transaction.
///
/// Note: `reth_e2e_test_utils::transaction::TransactionTestContext::deploy_tx_bytes`
/// sets `to = Some(TxKind::Call(Address::random()))` instead of `TxKind::Create`,
/// so it does NOT actually deploy a contract.  We build the transaction ourselves.
async fn create_deploy_tx_bytes(
    chain_id: u64,
    gas_limit: u64,
    init_code: Bytes,
    wallet: PrivateKeySigner,
) -> Bytes {
    let req = TransactionRequest {
        to: Some(TxKind::Create),
        gas: Some(gas_limit),
        max_fee_per_gas: Some(20_000_000_000u128),
        max_priority_fee_per_gas: Some(20_000_000_000u128),
        nonce: Some(0),
        chain_id: Some(chain_id),
        value: Some(U256::ZERO),
        input: TransactionInput { input: None, data: Some(init_code) },
        ..Default::default()
    };
    let signer = EthereumWallet::from(wallet);
    let envelope =
        <TransactionRequest as TransactionBuilder<Ethereum>>::build(req, &signer).await.unwrap();
    Bytes::from(envelope.encoded_2718())
}

/// Chain spec matching the one in reth's own rocksdb e2e tests (Cancun-activated).
fn test_chain_spec() -> Arc<ChainSpec> {
    Arc::new(
        ChainSpecBuilder::default()
            .chain(MAINNET.chain)
            .genesis(
                serde_json::from_str(include_str!("assets/genesis.json"))
                    .expect("failed to parse genesis.json"),
            )
            .cancun_activated()
            .build(),
    )
}

fn test_attributes_generator(timestamp: u64) -> EthPayloadBuilderAttributes {
    let attributes = alloy_rpc_types_engine::PayloadAttributes {
        timestamp,
        prev_randao: B256::ZERO,
        suggested_fee_recipient: Address::ZERO,
        withdrawals: Some(vec![]),
        parent_beacon_block_root: Some(B256::ZERO),
    };
    EthPayloadBuilderAttributes::new(B256::ZERO, attributes)
}

/// Poll `eth_getLogs` until a log with `topic` appears in `block_hash` or
/// `timeout` expires.  Mirrors the `retryForever` pattern in evmengine, except
/// we also fail on persistent empty results to surface the bug.
async fn wait_for_log(
    client: &impl ClientT,
    block_hash: B256,
    topic: B256,
    timeout: Duration,
) -> Option<Log> {
    let start = std::time::Instant::now();
    loop {
        let filter = serde_json::json!({
            "blockHash": format!("{block_hash:#x}"),
            "topics":    [[format!("{topic:#x}")]],
        });

        let logs: Vec<Log> = client
            .request("eth_getLogs", [filter])
            .await
            .expect("eth_getLogs must not return an RPC error");

        if !logs.is_empty() {
            return logs.into_iter().next();
        }
        if start.elapsed() >= timeout {
            return None;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Wait until a tx hash appears in the mempool (prevents advance_block racing).
async fn wait_for_pending_tx(client: &impl ClientT, tx_hash: B256, timeout: Duration) {
    let start = std::time::Instant::now();
    loop {
        let tx: Option<alloy_rpc_types_eth::Transaction> =
            client.request("eth_getTransactionByHash", [tx_hash]).await.unwrap();
        if tx.is_some() {
            return;
        }
        assert!(start.elapsed() < timeout, "timed out waiting for tx in mempool");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// **Core regression test.**
///
/// With `--storage.v2` enabled, `eth_getLogs(blockHash)` must return the log
/// that was emitted by the contract-deploy transaction after the block is
/// committed through the Engine API
/// (`engine_newPayloadV4` → `engine_forkchoiceUpdatedV3`).
///
/// Failure here means mito-reth with `--storage.v2` would silently drop EVM
/// events from the Cosmos consensus layer, producing an AppHash mismatch.
#[tokio::test(flavor = "multi_thread")]
async fn test_storage_v2_eth_get_logs_after_engine_api_block() -> Result<()> {
    reth_tracing::init_test_tracing();

    let chain_spec = test_chain_spec();
    let chain_id = chain_spec.chain().id();

    let (mut nodes, _) =
        E2ETestSetupBuilder::<EthereumNode, _>::new(1, chain_spec, test_attributes_generator)
            .with_storage_v2() // ← the flag under test
            .with_tree_config_modifier(|cfg| cfg.with_persistence_threshold(0))
            .build()
            .await?;

    let node = &mut nodes[0];
    let client = node.rpc_client().expect("RPC client");

    // 1. Deploy a contract that emits TRANSFER_TOPIC in its constructor.
    let signer = Wallet::new(1).with_chain_id(chain_id).wallet_gen().remove(0);
    let raw_tx = create_deploy_tx_bytes(chain_id, 200_000, log_emitter_init_code(), signer).await;

    let tx_hash = node.rpc.inject_tx(raw_tx).await?;
    wait_for_pending_tx(&client, tx_hash, Duration::from_secs(10)).await;

    // 2. Build + submit payload via Engine API, make it canonical.
    //    `advance_block` wraps: engine_newPayloadV3/V4 + engine_forkchoiceUpdatedV3.
    let payload = node.advance_block().await?;
    let block_hash = payload.block().hash();
    let block_number = payload.block().number();

    assert_eq!(block_number, 1, "first mined block must be #1");

    // Verify the deploy tx is included.
    let receipt: Option<TransactionReceipt> =
        client.request("eth_getTransactionReceipt", [tx_hash]).await?;
    assert!(receipt.is_some(), "deploy tx receipt must exist after mining");
    assert!(receipt.unwrap().status(), "deploy tx must succeed");

    // 3. eth_getLogs(blockHash, topic=TRANSFER_TOPIC) — the exact call that
    //    octane/evmengine makes in fetchProcEvents / FilterLogs.
    let log = wait_for_log(&client, block_hash, TRANSFER_TOPIC, Duration::from_secs(5)).await;

    assert!(
        log.is_some(),
        "eth_getLogs returned [] for block {block_hash:#x} with --storage.v2.\n\
         This is the AppHash mismatch root cause: evmengine FilterLogs silently \
         receives no events, skips the Cosmos state update, and the multistore \
         hash diverges from the network."
    );

    let log = log.unwrap();
    assert_eq!(log.block_hash, Some(block_hash), "log must reference the correct block");
    assert_eq!(
        log.inner.topics().first().copied(),
        Some(TRANSFER_TOPIC),
        "first topic must be the Transfer event signature"
    );

    Ok(())
}

/// **Baseline / sanity check** — same test without `--storage.v2`.
///
/// If this passes and `test_storage_v2_*` fails, the regression is definitively
/// isolated to the storage.v2 code path.
#[tokio::test(flavor = "multi_thread")]
async fn test_default_storage_eth_get_logs_baseline() -> Result<()> {
    reth_tracing::init_test_tracing();

    let chain_spec = test_chain_spec();
    let chain_id = chain_spec.chain().id();

    let (mut nodes, _) =
        E2ETestSetupBuilder::<EthereumNode, _>::new(1, chain_spec, test_attributes_generator)
            // no .with_storage_v2()
            .build()
            .await?;

    let node = &mut nodes[0];
    let client = node.rpc_client().expect("RPC client");

    let signer = Wallet::new(1).with_chain_id(chain_id).wallet_gen().remove(0);
    let raw_tx = create_deploy_tx_bytes(chain_id, 200_000, log_emitter_init_code(), signer).await;

    let tx_hash = node.rpc.inject_tx(raw_tx).await?;
    wait_for_pending_tx(&client, tx_hash, Duration::from_secs(10)).await;

    let payload = node.advance_block().await?;
    let block_hash = payload.block().hash();

    let receipt: Option<TransactionReceipt> =
        client.request("eth_getTransactionReceipt", [tx_hash]).await?;
    assert!(receipt.is_some(), "baseline: deploy tx receipt must exist");
    assert!(receipt.unwrap().status(), "baseline: deploy tx must succeed");

    let log = wait_for_log(&client, block_hash, TRANSFER_TOPIC, Duration::from_secs(5)).await;

    assert!(log.is_some(), "baseline: eth_getLogs returned [] — unexpected");

    Ok(())
}
