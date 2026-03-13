//! `ress` RLPx subprotocol installer.
//!
//! This module is compiled only when the `ress` feature is enabled.  It
//! requires `reth-ress-protocol` and `reth-ress-provider` from the Mitosis
//! fork to be API-compatible with the upstream reth v1.11.3 crates they
//! depend on.
//!
//! ## Status: remaining fork point
//!
//! The `reth-ress-protocol` and `reth-ress-provider` crates are entirely new
//! code that does not exist in upstream reth — they are not a diff of an
//! existing upstream crate.  Their dependencies (`reth-eth-wire`,
//! `reth-network`, …) are all standard upstream crates, so a port should be
//! straightforward once the internal API changes between v1.6.0 and v1.11.3
//! are resolved.
//!
//! Track the porting work in a child issue of MIT-307.

#![cfg(feature = "ress")]

use reth_ethereum_primitives::EthPrimitives;
use reth_evm::ConfigureEvm;
use reth_network_api::{FullNetwork, NetworkProtocols};
use reth_node_api::BeaconConsensusEngineEvent;
use reth_provider::providers::{BlockchainProvider, ProviderNodeTypes};
use reth_ress_protocol::{NodeType, ProtocolState, RessProtocolHandler};
use reth_ress_provider::{maintain_pending_state, PendingState, RethRessProtocolProvider};
use reth_tasks::TaskExecutor;
use reth_tokio_util::EventStream;
use tokio::sync::mpsc;
use tracing::*;

/// CLI arguments for the ress subprotocol.
#[derive(Debug, Clone, clap::Args)]
pub struct RessArgs {
    /// Enable the ress stateless-sync subprotocol.
    #[arg(long = "ress.enabled", default_value_t = false)]
    pub enabled: bool,

    /// Maximum window (in blocks) for which witnesses are served.
    #[arg(long = "ress.max-witness-window", default_value_t = 64)]
    pub max_witness_window: u64,

    /// Maximum number of witnesses fetched in parallel.
    #[arg(long = "ress.witness-max-parallel", default_value_t = 8)]
    pub witness_max_parallel: usize,

    /// LRU cache size for witnesses.
    #[arg(long = "ress.witness-cache-size", default_value_t = 256)]
    pub witness_cache_size: usize,

    /// Maximum simultaneous active ress connections.
    #[arg(long = "ress.max-active-connections", default_value_t = 10)]
    pub max_active_connections: usize,
}

/// Install the `ress` subprotocol on a running node.
pub fn install_ress_subprotocol<P, E, N>(
    args: RessArgs,
    provider: BlockchainProvider<P>,
    evm_config: E,
    network: N,
    task_executor: TaskExecutor,
    engine_events: EventStream<BeaconConsensusEngineEvent<EthPrimitives>>,
) -> eyre::Result<()>
where
    P: ProviderNodeTypes<Primitives = EthPrimitives>,
    E: ConfigureEvm<Primitives = EthPrimitives> + Clone + 'static,
    N: FullNetwork + NetworkProtocols,
{
    if !args.enabled {
        return Ok(());
    }

    info!(target: "mi-reth::ress", "Installing ress subprotocol");
    let pending_state = PendingState::default();

    task_executor.spawn(maintain_pending_state(
        engine_events,
        provider.clone(),
        pending_state.clone(),
    ));

    let (tx, mut rx) = mpsc::unbounded_channel();
    let protocol_provider = RethRessProtocolProvider::new(
        provider,
        evm_config,
        Box::new(task_executor.clone()),
        args.max_witness_window,
        args.witness_max_parallel,
        args.witness_cache_size,
        pending_state,
    )?;

    network.add_rlpx_sub_protocol(
        RessProtocolHandler {
            provider: protocol_provider,
            node_type: NodeType::Stateful,
            peers_handle: network.peers_handle().clone(),
            max_active_connections: args.max_active_connections,
            state: ProtocolState::new(tx),
        }
        .into_rlpx_sub_protocol(),
    );

    task_executor.spawn(async move {
        while let Some(event) = rx.recv().await {
            trace!(target: "mi-reth::ress", ?event, "Received ress event");
        }
    });

    info!(target: "mi-reth::ress", "Ress subprotocol enabled");
    Ok(())
}
