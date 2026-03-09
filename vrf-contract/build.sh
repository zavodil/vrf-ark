#!/bin/bash
set -e

echo "Building vrf-contract..."

cargo near build non-reproducible-wasm

echo ""
echo "Build complete:"
echo "  target/near/vrf_contract.wasm"


# near contract deploy coin-flip-vrf.testnet use-file /Users/alice/projects/near-offshore/wasi-examples/vrf-ark/vrf-contract/target/near/vrf_contract.wasm with-init-call new json-args '{"outlayer_contract_id":"outlayer.testnet","project_id":"zavodil2.testnet/vrf-ark","vrf_pubkey_hex":"c0c5aeae1688c2f7cc2350aae1ee3ce96eddc0de84d678db873de48268f8cfa8"}' prepaid-gas '100.0 Tgas' attached-deposit '0 NEAR' network-config testnet sign-with-legacy-keychain send
