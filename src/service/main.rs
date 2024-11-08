use ic_cdk::{query, update};
use std::cell::RefCell;

use alloy::{
    network::EthereumWallet,
    primitives::{address, U256},
    providers::{Provider, ProviderBuilder},
    signers::Signer,
    sol,
    transports::icp::IcpConfig,
};

use ic_cdk::export::candid::{CandidType, Deserialize};
use ic_cdk::storage;
use ic_cdk_macros::*;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{create_icp_signer, get_rpc_service_sepolia};

// Define the interval in seconds (e.g., 1 day = 86400 seconds)
const TRANSFER_INTERVAL_SECONDS: u64 = 86400;

// Store the last transfer time
#[derive(CandidType, Deserialize, Default)]
struct SubscriptionState {
    last_transfer_time: u64,
}

thread_local! {
    static NONCE: RefCell<Option<u64>> = const { RefCell::new(None) };
}

// Codegen from ABI file to interact with the contract.
sol!(
    #[allow(missing_docs, clippy::too_many_arguments)]
    #[sol(rpc)]
    USDC,
    "src/abi/USDC.json"
);

/// Get the Ethereum address of the backend canister.
#[query]
async fn get_address() -> Result<String, String> {
    let signer = create_icp_signer().await;
    let address = signer.address();
    Ok(address.to_string())
}

/// Transfer USDC
#[ic_cdk::update]
async fn transfer_usdc() -> Result<String, String> {
    let from_address = address!("63A0bfd6a5cdCF446ae12135E2CD86b908659563");

    // Setup signer
    let signer = create_icp_signer().await;
    let address = signer.address();

    // Setup provider
    let wallet = EthereumWallet::from(signer);
    let rpc_service = get_rpc_service_sepolia();
    let config = IcpConfig::new(rpc_service);
    let provider = ProviderBuilder::new()
        .with_gas_estimation()
        .wallet(wallet)
        .on_icp(config);

    // Attempt to get nonce from thread-local storage
    let maybe_nonce = NONCE.with_borrow(|maybe_nonce| {
        // If a nonce exists, the next nonce to use is latest nonce + 1
        maybe_nonce.map(|nonce| nonce + 1)
    });

    // If no nonce exists, get it from the provider
    let nonce = if let Some(nonce) = maybe_nonce {
        nonce
    } else {
        provider.get_transaction_count(address).await.unwrap_or(0)
    };

    let contract = USDC::new(
        address!("1c7d4b196cb0c7b01d743fbc6116a902379c7238"),
        provider.clone(),
    );

    match contract
        .transferFrom(from_address, address, U256::from(1))
        .nonce(nonce)
        .chain_id(11155111)
        .from(address)
        .send()
        .await
    {
        Ok(builder) => {
            let node_hash = *builder.tx_hash();
            let tx_response = provider.get_transaction_by_hash(node_hash).await.unwrap();

            match tx_response {
                Some(tx) => {
                    // The transaction has been mined and included in a block, the nonce
                    // has been consumed. Save it to thread-local storage. Next transaction
                    // for this address will use a nonce that is = this nonce + 1
                    NONCE.with_borrow_mut(|nonce| {
                        *nonce = Some(tx.nonce);
                    });
                    Ok(format!("{:?}", tx))
                }
                None => Err("Could not get transaction.".to_string()),
            }
        }
        Err(e) => Err(format!("{:?}", e)),
    }
}

#[update]
async fn transfer_usdc_periodically() -> Result<String, String> {
    let state = storage::get_mut::<SubscriptionState>();

    // Get the current time in seconds since UNIX_EPOCH
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    // Check if the interval has passed
    if current_time >= state.last_transfer_time + TRANSFER_INTERVAL_SECONDS {
        // Call the `transfer_usdc` function
        let result = transfer_usdc().await;

        // Update the last transfer time if successful
        if result.is_ok() {
            state.last_transfer_time = current_time;
        }
        result
    } else {
        Err("Transfer not yet due.".to_string())
    }
}
