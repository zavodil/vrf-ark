mod types;

use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::{env, ext_contract, log, near_bindgen, AccountId, NearToken, Promise, PromiseError};

use types::{CoinSide, VrfEntry, VrfResponse};

/// Minimum deposit to cover OutLayer execution cost
const MIN_DEPOSIT: u128 = 10_000_000_000_000_000_000_000; // 0.01 NEAR

/// Fixed gas for callback (ed25519_verify costs ~26 TGas)
const CALLBACK_GAS: u64 = 50_000_000_000_000; // 50 TGas

/// External contract interface for OutLayer
#[ext_contract(ext_outlayer)]
#[allow(dead_code)]
trait OutLayer {
    fn request_execution(
        &mut self,
        source: near_sdk::serde_json::Value,
        resource_limits: Option<near_sdk::serde_json::Value>,
        input_data: Option<String>,
        secrets_ref: Option<near_sdk::serde_json::Value>,
        response_format: Option<String>,
        payer_account_id: Option<AccountId>,
    );
}

/// External contract interface for self callbacks
#[ext_contract(ext_self)]
#[allow(dead_code)]
trait ExtSelf {
    fn on_vrf_result(
        &mut self,
        player: AccountId,
        choice: CoinSide,
        #[callback_result] result: Result<Option<VrfResponse>, PromiseError>,
    ) -> String;
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct VrfCoinFlipContract {
    outlayer_contract_id: AccountId,
    /// OutLayer project ID (e.g. "alice.near/vrf-ark")
    project_id: String,
    /// VRF public key bytes (set once via set_vrf_pubkey)
    vrf_pubkey: Vec<u8>,
}

impl Default for VrfCoinFlipContract {
    fn default() -> Self {
        env::panic_str("Contract must be initialized with new()")
    }
}

#[near_bindgen]
impl VrfCoinFlipContract {
    /// Initialize with VRF public key
    #[init]
    pub fn new(outlayer_contract_id: AccountId, project_id: String, vrf_pubkey_hex: String) -> Self {
        assert!(!project_id.is_empty(), "project_id cannot be empty");
        let vrf_pubkey = hex::decode(&vrf_pubkey_hex)
            .unwrap_or_else(|_| env::panic_str("Invalid VRF pubkey hex"));
        assert_eq!(vrf_pubkey.len(), 32, "VRF pubkey must be 32 bytes");

        Self {
            outlayer_contract_id,
            project_id,
            vrf_pubkey,
        }
    }

    /// Update VRF public key (owner only)
    pub fn set_vrf_pubkey(&mut self, vrf_pubkey_hex: String) {
        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "Only contract owner can update VRF pubkey"
        );
        self.vrf_pubkey = hex::decode(&vrf_pubkey_hex)
            .unwrap_or_else(|_| env::panic_str("Invalid VRF pubkey hex"));
        assert_eq!(self.vrf_pubkey.len(), 32, "VRF pubkey must be 32 bytes");
    }

    /// Get current VRF public key
    pub fn get_vrf_pubkey(&self) -> String {
        hex::encode(&self.vrf_pubkey)
    }

    /// Get project ID
    pub fn get_project_id(&self) -> &str {
        &self.project_id
    }

    /// Flip a coin with verifiable randomness
    ///
    /// Uses OutLayer VRF instead of plain random — result includes cryptographic proof.
    /// The callback verifies the proof on-chain using ed25519_verify.
    #[payable]
    pub fn flip_coin(&mut self, choice: CoinSide) -> Promise {
        assert!(
            !self.vrf_pubkey.is_empty(),
            "VRF pubkey not set. Call new() or set_vrf_pubkey() first."
        );

        let player = env::predecessor_account_id();
        let attached = env::attached_deposit().as_yoctonear();

        assert!(
            attached >= MIN_DEPOSIT,
            "Minimum deposit is 0.01 NEAR to pay for OutLayer execution"
        );

        log!(
            "Player {} chose {:?}. Requesting VRF from OutLayer",
            player,
            choice
        );

        let source = near_sdk::serde_json::json!({
            "Project": {
                "project_id": self.project_id,
            }
        });

        let resource_limits = near_sdk::serde_json::json!({
            "max_instructions": 10000000000u64,
            "max_memory_mb": 128u32,
            "max_execution_seconds": 60u64
        });

        // Request VRF with seed "coin-flip", max=1 (0=Heads, 1=Tails)
        let input = r#"{"seed":"coin-flip","max":1}"#.to_string();

        ext_outlayer::ext(self.outlayer_contract_id.clone())
            .with_attached_deposit(NearToken::from_yoctonear(attached))
            .with_unused_gas_weight(1)
            .request_execution(
                source,
                Some(resource_limits),
                Some(input),
                None,
                Some("Json".to_string()),
                Some(player.clone()),
            )
            .then(
                ext_self::ext(env::current_account_id())
                    .with_static_gas(near_sdk::Gas::from_gas(CALLBACK_GAS))
                    .on_vrf_result(player, choice),
            )
    }

    /// Callback: verify VRF proof and determine result
    #[private]
    pub fn on_vrf_result(
        &mut self,
        player: AccountId,
        choice: CoinSide,
        #[callback_result] result: Result<Option<VrfResponse>, PromiseError>,
    ) -> String {
        match result {
            Ok(Some(vrf_response)) => {
                let entry = &vrf_response.results[0];
                log!("VRF result: value={}, alpha={}", entry.value, entry.alpha);

                // Verify VRF proof on-chain
                if !self.verify_vrf_proof(entry) {
                    env::panic_str("VRF proof verification failed!");
                }
                log!("VRF proof verified on-chain!");

                // Extract random number
                let random_number = entry
                    .value
                    .as_u64()
                    .unwrap_or_else(|| env::panic_str("Expected numeric VRF value"));

                let result_side = if random_number == 0 {
                    CoinSide::Heads
                } else {
                    CoinSide::Tails
                };

                if choice == result_side {
                    log!(
                        "Player {} WON! Choice: {:?}, Result: {:?} (verified)",
                        player, choice, result_side
                    );
                    format!(
                        "You won! Result: {:?}, Choice: {:?}. VRF proof verified on-chain.",
                        result_side, choice
                    )
                } else {
                    log!(
                        "Player {} LOST. Choice: {:?}, Result: {:?} (verified)",
                        player, choice, result_side
                    );
                    format!(
                        "You lost. Result: {:?}, Choice: {:?}. VRF proof verified on-chain.",
                        result_side, choice
                    )
                }
            }

            Ok(None) => {
                log!("OutLayer execution failed - received None");
                env::panic_str("OutLayer execution failed")
            }

            Err(promise_error) => {
                log!("Promise error: {:?}", promise_error);
                env::panic_str(&format!("Promise error: {:?}", promise_error))
            }
        }
    }

    /// Verify VRF proof using ed25519_verify
    ///
    /// Checks: ed25519_verify(vrf_pubkey, alpha.as_bytes(), signature_bytes) == true
    fn verify_vrf_proof(&self, entry: &VrfEntry) -> bool {
        let sig_vec = match hex::decode(&entry.signature_hex) {
            Ok(bytes) if bytes.len() == 64 => bytes,
            _ => {
                log!("Invalid signature hex");
                return false;
            }
        };
        let signature: &[u8; 64] = sig_vec.as_slice().try_into().unwrap();
        let pubkey: &[u8; 32] = self.vrf_pubkey.as_slice().try_into().unwrap();

        // NEAR native ed25519_verify: (signature, message, public_key)
        env::ed25519_verify(signature, entry.alpha.as_bytes(), pubkey)
    }
}
