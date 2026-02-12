use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Single VRF result entry from the WASI module
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct VrfEntry {
    pub value: serde_json::Value,
    pub signature_hex: String,
    pub alpha: String,
}

/// Full VRF response from the WASI module
#[derive(Serialize, Deserialize, JsonSchema, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct VrfResponse {
    pub results: Vec<VrfEntry>,
    pub verification: serde_json::Value,
}

/// Player's coin flip choice
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone, Copy, Debug, PartialEq)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub enum CoinSide {
    Heads, // 0
    Tails, // 1
}
