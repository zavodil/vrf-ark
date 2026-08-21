# VRF Ark - Verifiable Random Function Example

Verifiable randomness for NEAR smart contracts using OutLayer VRF.

Unlike `random-example` (plain random, unverifiable), this example produces **cryptographic proof** that the random number wasn't manipulated. Anyone can verify the proof on-chain.

## How It Works

```
User calls flip_coin(Heads)
  --> NEAR Contract calls OutLayer
    --> Worker executes vrf-example.wasm (wasm32-wasip2)
      --> WASM calls outlayer::vrf::random("coin-flip")
        --> Worker host function: alpha = "vrf:{request_id}:{sender_id}:coin-flip"
          --> Keystore (TEE): Ed25519 sign(vrf_key, alpha)
        <-- Returns: output (SHA256 of signature), signature (proof), alpha
      <-- WASM outputs JSON with value + proof
    <-- OutLayer returns result to contract
  --> Contract callback: ed25519_verify(vrf_pubkey, alpha, signature)
  --> Proof valid! Result is trustworthy.
```

## VRF vs Plain Random

| Feature | random-example | vrf-example |
|---------|-----------|---------|
| Target | wasm32-wasip1 | wasm32-wasip2 |
| Source | `rand` crate (WASI random) | OutLayer SDK `vrf::random()` |
| Proof | None | Ed25519 signature |
| Verifiable | No | Yes (on-chain) |
| Deterministic | No (different each run) | Yes (same key + same seed = same output) |
| Manipulation | Worker could lie | Impossible without VRF private key |

## Input Format

```json
{
  "seed": "my-unique-seed",
  "count": 1,
  "max": 100
}
```

- `seed` (required) — arbitrary string, combined with request_id by host
- `count` (optional, default 1) — how many VRF outputs (max 10)
- `max` (optional) — if set, maps output to 0..=max; otherwise returns raw hex

## Output Format

```json
{
  "results": [
    {
      "value": 42,
      "signature_hex": "abcd...1234",
      "alpha": "vrf:12345:alice.near:my-unique-seed"
    }
  ],
  "verification": {
    "steps": ["1. Get VRF public key...", "2. Verify signature...", "..."],
    "pubkey_endpoint": "https://api.outlayer.ai/vrf/pubkey",
    "near_code": "let valid = env::ed25519_verify(...);"
  }
}
```

## Building

```bash
# WASI module (wasm32-wasip2 required for VRF host functions)
rustup target add wasm32-wasip2
cargo build --target wasm32-wasip2 --release

# Output: target/wasm32-wasip2/release/vrf-example.wasm
```

## On-Chain Verification

### 1. Get VRF Public Key

```bash
curl https://api.outlayer.ai/vrf/pubkey
# {"vrf_public_key_hex": "a1b2c3..."}
```

### 2. Verify in NEAR Contract

```rust
use near_sdk::env;

fn verify_vrf(
    vrf_pubkey: &[u8],    // 32 bytes from /vrf/pubkey
    alpha: &str,           // take it FROM the output — see the note below
    signature: &[u8],      // 64 bytes from signature_hex
) -> bool {
    env::ed25519_verify(signature, alpha.as_bytes(), vrf_pubkey)
}
```

### 3. Verify Off-Chain (Node.js)

```javascript
import { verify } from '@noble/ed25519';

const pubkey = Buffer.from(vrfPubkeyHex, 'hex');
const signature = Buffer.from(signatureHex, 'hex');
const message = Buffer.from(alpha); // exactly as returned, never rebuilt

const valid = await verify(signature, message, pubkey);
console.log('VRF proof valid:', valid);
```

### Use the `alpha` you were given — do not rebuild it

The signature covers those exact bytes, so a verifier that assembles its own
string has to match the worker's format character for character or the check
fails for a proof that is perfectly good.

Today that format is `vrf:{request_id}:{sender_id}:{user_seed}`, where
`sender_id` is the account that PAID for the call — the payment key's owner
over HTTPS, the transaction's sender on chain. It is deliberately not the name
the guest runs under: a wallet using Agent Connect with `use_bound_identity`
acts as its bound account, and if the alpha followed that, the same module with
the same flag would draw a different random stream depending on which door
started it.

`request_id` is unique per call regardless, so this changes no guarantee about
the randomness itself. What it buys is that one agent gets one answer to "whose
domain is this", and that the string stays reconstructible from the paying
account if you ever need to reason about it offline.

None of that matters if the returned `alpha` is passed through unchanged, which
is why every example here does exactly that.

## Example Contract

See [vrf-contract/](vrf-contract/) for a coin flip contract that:
1. Requests VRF from OutLayer
2. Verifies the proof on-chain using `env::ed25519_verify`
3. Uses verified random number to determine win/loss

```bash
cd vrf-contract
cargo near build non-reproducible-wasm
```

## Architecture

```
vrf-example (WASI module, wasm32-wasip2)
  Uses: outlayer::vrf::random(seed)
  Returns: { value, signature_hex, alpha }

vrf-contract (NEAR contract)
  Calls: OutLayer request_execution
  Verifies: env::ed25519_verify(vrf_pubkey, alpha, signature)
  Result: Provably fair coin flip
```

## Security

- **request_id** is assigned by the OutLayer contract (sequential, on-chain). The WASM module cannot choose or predict it.
- **alpha** = `"vrf:{request_id}:{user_seed}"` — the host auto-prepends request_id, WASM only controls user_seed.
- **VRF key** is derived from the keystore's master secret (shared via MPC CKD). All approved keystores produce the same output.
- **Deterministic**: same key + same alpha = same signature = same random output. No re-rolling.
- **Proof**: Ed25519 signature over alpha. Verify with `env::ed25519_verify` (native NEAR, no extra deps).

## License

MIT OR Apache-2.0, at your option — see `LICENSE-MIT` and `LICENSE-APACHE`.
