use outlayer::vrf;
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

#[derive(Deserialize)]
struct Input {
    /// User seed for VRF (arbitrary string)
    seed: String,
    /// How many random numbers to generate (default: 1)
    #[serde(default = "default_count")]
    count: u32,
    /// Range max (inclusive) — maps VRF output to 0..=max (default: no mapping, return raw hex)
    #[serde(default)]
    max: Option<u32>,
}

fn default_count() -> u32 {
    1
}

#[derive(Serialize)]
struct VrfEntry {
    /// Random output: hex string (if no max) or number (if max specified)
    value: serde_json::Value,
    /// Ed25519 signature (hex) — the proof
    signature_hex: String,
    /// Full alpha: "vrf:{request_id}:{seed}" — for verification
    alpha: String,
}

#[derive(Serialize)]
struct Output {
    /// VRF results (one per requested count)
    results: Vec<VrfEntry>,
    /// How to verify on-chain
    verification: Verification,
}

#[derive(Serialize)]
struct Verification {
    /// Step-by-step verification instructions
    steps: Vec<String>,
    /// VRF public key endpoint
    pubkey_endpoint: String,
    /// NEAR contract verification code
    near_code: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input_string = String::new();
    io::stdin().read_to_string(&mut input_string)?;

    let input: Input = serde_json::from_str(&input_string)?;

    if input.count == 0 || input.count > 10 {
        return Err("count must be 1..10".into());
    }

    let mut results = Vec::new();

    for i in 0..input.count {
        // Each call uses a unique sub-seed: "{seed}:{i}"
        let sub_seed = if input.count == 1 {
            input.seed.clone()
        } else {
            format!("{}:{}", input.seed, i)
        };

        let vrf_output = vrf::random(&sub_seed)
            .map_err(|e| format!("VRF failed: {}", e))?;

        let value = match input.max {
            Some(max) => {
                // Map first 4 bytes of output to 0..=max
                let bytes = hex_to_u32(&vrf_output.output_hex);
                let mapped = (bytes as u64 * (max as u64 + 1) / (u32::MAX as u64 + 1)) as u32;
                serde_json::Value::Number(serde_json::Number::from(mapped))
            }
            None => {
                serde_json::Value::String(vrf_output.output_hex.clone())
            }
        };

        results.push(VrfEntry {
            value,
            signature_hex: vrf_output.signature_hex,
            alpha: vrf_output.alpha,
        });
    }

    let output = Output {
        results,
        verification: Verification {
            steps: vec![
                "1. Get VRF public key: GET https://api.outlayer.fastnear.com/vrf/pubkey".into(),
                "2. For each result, verify: ed25519_verify(vrf_pubkey, alpha.as_bytes(), signature)".into(),
                "3. Confirm output = SHA256(signature) matches the value".into(),
                "4. Alpha contains request_id (from on-chain event) — cannot be forged".into(),
            ],
            pubkey_endpoint: "https://api.outlayer.fastnear.com/vrf/pubkey".into(),
            near_code: concat!(
                "let valid = env::ed25519_verify(",
                "&vrf_pubkey_bytes, ",
                "alpha.as_bytes(), ",
                "&signature_bytes);"
            ).into(),
        },
    };

    let json = serde_json::to_string(&output)?;
    print!("{}", json);
    io::stdout().flush()?;

    Ok(())
}

/// Parse first 4 bytes of hex string as u32
fn hex_to_u32(hex: &str) -> u32 {
    let bytes: Vec<u8> = (0..8.min(hex.len()))
        .step_by(2)
        .filter_map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect();
    let mut buf = [0u8; 4];
    for (i, b) in bytes.iter().take(4).enumerate() {
        buf[i] = *b;
    }
    u32::from_be_bytes(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_to_u32() {
        assert_eq!(hex_to_u32("ffffffff"), u32::MAX);
        assert_eq!(hex_to_u32("00000000"), 0);
        assert_eq!(hex_to_u32("80000000"), 0x80000000);
    }

    #[test]
    fn test_input_parsing() {
        let json = r#"{"seed":"test"}"#;
        let input: Input = serde_json::from_str(json).unwrap();
        assert_eq!(input.seed, "test");
        assert_eq!(input.count, 1);
        assert!(input.max.is_none());
    }

    #[test]
    fn test_input_with_max() {
        let json = r#"{"seed":"test","max":100,"count":3}"#;
        let input: Input = serde_json::from_str(json).unwrap();
        assert_eq!(input.count, 3);
        assert_eq!(input.max, Some(100));
    }
}
