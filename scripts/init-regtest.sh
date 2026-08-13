#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
state_dir="${REGTEST_STATE_DIR:-$project_dir/.regtest}"
fixture="$state_dir/test-data.json"
cli=(
  bitcoin-cli
  -regtest
  -rpcconnect="${BITCOIN_RPC_HOST:-127.0.0.1}"
  -rpcport="${BITCOIN_RPC_PORT:-18443}"
  -rpcuser="${BITCOIN_RPC_USER:-pg_bitcoin}"
  -rpcpassword="${BITCOIN_RPC_PASSWORD:-pg_bitcoin}"
)

mkdir -p "$state_dir"
touch "$state_dir/.pg-bitcoin-regtest"

# Fixed regtest keys. Never use them on a public network.
funding_wif="cMahea7zqjxrtgAbB7LSGbcQUr1uX1ojuat9jZodMN87JcbXMTcA"
recipient_wifs=(
  "cMahea7zqjxrtgAbB7LSGbcQUr1uX1ojuat9jZodMN87K7XCyj5v"
  "cMahea7zqjxrtgAbB7LSGbcQUr1uX1ojuat9jZodMN87KcLPVfXz"
  "cMahea7zqjxrtgAbB7LSGbcQUr1uX1ojuat9jZodMN87L7FgDCKE"
  "cMahea7zqjxrtgAbB7LSGbcQUr1uX1ojuat9jZodMN87Lc8ycuM4"
  "cMahea7zqjxrtgAbB7LSGbcQUr1uX1ojuat9jZodMN87M73ZA41f"
)
mining_wif="cMahea7zqjxrtgAbB7LSGbcQUr1uX1ojuat9jZodMN87MbvFSpLf"

descriptor_for() {
  "${cli[@]}" getdescriptorinfo "wpkh($1)" | jq -er '.descriptor'
}

address_for() {
  "${cli[@]}" deriveaddresses "$1" | jq -er '.[0]'
}

scripthash_for() {
  hash="$(printf '%s' "$1" | xxd -r -p | sha256sum)"
  hash="${hash%% *}"
  reversed=""
  for ((offset = ${#hash}; offset > 0; offset -= 2)); do
    reversed+="${hash:$((offset - 2)):2}"
  done
  printf '%s\n' "$reversed"
}

if [[ -f "$fixture" ]]; then
  if [[ "$(jq -r '.schema_version // 0' "$fixture")" != "3" ]]; then
    printf 'Outdated fixture at %s; restart with --reset.\n' "$fixture" >&2
    exit 1
  fi
  txid="$(jq -er '.transactions.confirmed.txid' "$fixture")"
  block_hash="$(jq -er '.transactions.confirmed.block_hash' "$fixture")"
  mempool_txid="$(jq -er '.transactions.mempool.txid' "$fixture")"
  if "${cli[@]}" getrawtransaction "$txid" true "$block_hash" >/dev/null 2>&1 \
    && "${cli[@]}" getmempoolentry "$mempool_txid" >/dev/null 2>&1; then
    jq . "$fixture"
    exit 0
  fi
  printf 'Stale fixture at %s; restart with --reset.\n' "$fixture" >&2
  exit 1
fi

height="$("${cli[@]}" getblockcount)"
if [[ "$height" != "0" ]]; then
  printf 'Regtest chain already has %s blocks but no fixture; restart with --reset.\n' "$height" >&2
  exit 1
fi

funding_address="$(address_for "$(descriptor_for "$funding_wif")")"
mining_address="$(address_for "$(descriptor_for "$mining_wif")")"
recipient_addresses=()
for wif in "${recipient_wifs[@]}"; do
  recipient_addresses+=("$(address_for "$(descriptor_for "$wif")")")
done

mocktime=1700000000
"${cli[@]}" setmocktime "$((mocktime + 600))" >/dev/null
"${cli[@]}" generatetoaddress 1 "$funding_address" 1000000 >/dev/null
for ((height = 2; height <= 101; height++)); do
  "${cli[@]}" setmocktime "$((mocktime + height * 600))" >/dev/null
  "${cli[@]}" generatetoaddress 1 "$mining_address" 1000000 >/dev/null
done

coinbase_block_hash="$("${cli[@]}" getblockhash 1)"
coinbase_block="$("${cli[@]}" getblock "$coinbase_block_hash" 2)"
coinbase_txid="$(jq -er '.tx[0].txid' <<<"$coinbase_block")"
coinbase_vout="$(jq -er --arg address "$funding_address" '.tx[0].vout[] | select(.scriptPubKey.address == $address) | .n' <<<"$coinbase_block")"
coinbase_script="$(jq -er --arg address "$funding_address" '.tx[0].vout[] | select(.scriptPubKey.address == $address) | .scriptPubKey.hex' <<<"$coinbase_block")"
coinbase_amount="$(jq -er --arg address "$funding_address" '.tx[0].vout[] | select(.scriptPubKey.address == $address) | .value' <<<"$coinbase_block")"

inputs="$(jq -cn --arg txid "$coinbase_txid" --argjson vout "$coinbase_vout" '[{txid: $txid, vout: $vout}]')"
outputs="$(jq -cn \
  --arg a1 "${recipient_addresses[0]}" \
  --arg a2 "${recipient_addresses[1]}" \
  --arg a3 "${recipient_addresses[2]}" \
  --arg a4 "${recipient_addresses[3]}" \
  --arg a5 "${recipient_addresses[4]}" \
  '[{($a1): 1}, {($a2): 2}, {($a3): 3}, {($a4): 4}, {($a5): 39.999}]')"
prevouts="$(jq -cn \
  --arg txid "$coinbase_txid" \
  --argjson vout "$coinbase_vout" \
  --arg script "$coinbase_script" \
  --argjson amount "$coinbase_amount" \
  '[{txid: $txid, vout: $vout, scriptPubKey: $script, amount: $amount}]')"
keys="$(jq -cn --arg key "$funding_wif" '[$key]')"

raw_tx="$("${cli[@]}" createrawtransaction "$inputs" "$outputs" 0 false)"
signed="$("${cli[@]}" signrawtransactionwithkey "$raw_tx" "$keys" "$prevouts")"
[[ "$(jq -er '.complete' <<<"$signed")" == "true" ]]
tx_hex="$(jq -er '.hex' <<<"$signed")"
decoded_tx="$("${cli[@]}" decoderawtransaction "$tx_hex")"
txid="$("${cli[@]}" sendrawtransaction "$tx_hex")"

"${cli[@]}" setmocktime "$((mocktime + 102 * 600))" >/dev/null
"${cli[@]}" generatetoaddress 1 "$mining_address" 1000000 >/dev/null
block_hash="$("${cli[@]}" getbestblockhash)"

mempool_inputs="$(jq -cn --arg txid "$txid" '[{txid: $txid, vout: 0}]')"
mempool_outputs="$(jq -cn --arg address "${recipient_addresses[1]}" '[{($address): 0.999}]')"
mempool_prevouts="$(jq -cn \
  --arg txid "$txid" \
  --arg script "$(jq -er '.vout[0].scriptPubKey.hex' <<<"$decoded_tx")" \
  '[{txid: $txid, vout: 0, scriptPubKey: $script, amount: 1}]')"
mempool_keys="$(jq -cn --arg key "${recipient_wifs[0]}" '[$key]')"
mempool_raw="$("${cli[@]}" createrawtransaction "$mempool_inputs" "$mempool_outputs" 0 false)"
mempool_signed="$("${cli[@]}" signrawtransactionwithkey "$mempool_raw" "$mempool_keys" "$mempool_prevouts")"
[[ "$(jq -er '.complete' <<<"$mempool_signed")" == "true" ]]
mempool_hex="$(jq -er '.hex' <<<"$mempool_signed")"
mempool_decoded="$("${cli[@]}" decoderawtransaction "$mempool_hex")"
mempool_txid="$("${cli[@]}" sendrawtransaction "$mempool_hex")"

funding_scripthash="$(scripthash_for "$coinbase_script")"
recipient_scripthashes=()
for ((vout = 0; vout < 5; vout++)); do
  script="$(jq -er --argjson vout "$vout" '.vout[] | select(.n == $vout) | .scriptPubKey.hex' <<<"$decoded_tx")"
  recipient_scripthashes+=("$(scripthash_for "$script")")
done

jq -n \
  --arg fulcrum_url "${FULCRUM_URL:-tcp://127.0.0.1:60401}" \
  --arg funding_address "$funding_address" \
  --arg funding_wif "$funding_wif" \
  --arg funding_scripthash "$funding_scripthash" \
  --arg a1 "${recipient_addresses[0]}" --arg w1 "${recipient_wifs[0]}" --arg s1 "${recipient_scripthashes[0]}" \
  --arg a2 "${recipient_addresses[1]}" --arg w2 "${recipient_wifs[1]}" --arg s2 "${recipient_scripthashes[1]}" \
  --arg a3 "${recipient_addresses[2]}" --arg w3 "${recipient_wifs[2]}" --arg s3 "${recipient_scripthashes[2]}" \
  --arg a4 "${recipient_addresses[3]}" --arg w4 "${recipient_wifs[3]}" --arg s4 "${recipient_scripthashes[3]}" \
  --arg a5 "${recipient_addresses[4]}" --arg w5 "${recipient_wifs[4]}" --arg s5 "${recipient_scripthashes[4]}" \
  --arg coinbase_txid "$coinbase_txid" \
  --argjson coinbase_vout "$coinbase_vout" \
  --arg coinbase_script "$coinbase_script" \
  --arg txid "$txid" \
  --arg tx_hex "$tx_hex" \
  --arg block_hash "$block_hash" \
  --argjson decoded_tx "$decoded_tx" \
  --arg mempool_txid "$mempool_txid" \
  --arg mempool_hex "$mempool_hex" \
  --argjson mempool_decoded "$mempool_decoded" \
  'def entry($name; $address; $wif; $scripthash; $history; $mempool; $balance; $utxos): {
    name: $name,
    address: $address,
    wif: $wif,
    scripthash: $scripthash,
    "scripthash.get_history": $history,
    "scripthash.get_mempool": $mempool,
    "scripthash.get_balance": $balance,
    "scripthash.listunspent": $utxos
  };
  def confirmed_utxo($vout; $value): {
    tx_hash: $txid,
    tx_pos: $vout,
    height: 102,
    value: $value,
    script_pubkey: $decoded_tx.vout[$vout].scriptPubKey.hex
  };
  def mempool_utxo: {
    tx_hash: $mempool_txid,
    tx_pos: 0,
    height: 0,
    value: 99900000,
    script_pubkey: $mempool_decoded.vout[0].scriptPubKey.hex
  };
  {
    schema_version: 3,
    network: "regtest",
    fulcrum_url: $fulcrum_url,
    addresses: [
      entry(
        "funding"; $funding_address; $funding_wif; $funding_scripthash;
        [
          {tx_hash: $coinbase_txid, height: 1},
          {tx_hash: $txid, height: 102}
        ];
        [];
        {confirmed: 0, unconfirmed: 0};
        []
      ) + {spent_outpoint: {tx_hash: $coinbase_txid, tx_pos: $coinbase_vout}},
      entry(
        "recipient_1"; $a1; $w1; $s1;
        [{tx_hash: $txid, height: 102}, {tx_hash: $mempool_txid, height: 0}];
        [{tx_hash: $mempool_txid, height: 0, fee: 100000}];
        {confirmed: 100000000, unconfirmed: -100000000};
        []
      ),
      entry(
        "recipient_2"; $a2; $w2; $s2;
        [{tx_hash: $txid, height: 102}, {tx_hash: $mempool_txid, height: 0}];
        [{tx_hash: $mempool_txid, height: 0, fee: 100000}];
        {confirmed: 200000000, unconfirmed: 99900000};
        [confirmed_utxo(1; 200000000), mempool_utxo]
      ),
      entry(
        "recipient_3"; $a3; $w3; $s3;
        [{tx_hash: $txid, height: 102}]; [];
        {confirmed: 300000000, unconfirmed: 0};
        [confirmed_utxo(2; 300000000)]
      ),
      entry(
        "recipient_4"; $a4; $w4; $s4;
        [{tx_hash: $txid, height: 102}]; [];
        {confirmed: 400000000, unconfirmed: 0};
        [confirmed_utxo(3; 400000000)]
      ),
      entry(
        "recipient_5"; $a5; $w5; $s5;
        [{tx_hash: $txid, height: 102}]; [];
        {confirmed: 3999900000, unconfirmed: 0};
        [confirmed_utxo(4; 3999900000)]
      )
    ],
    transactions: {
      confirmed: {
        txid: $txid,
        wtxid: $decoded_tx.hash,
        hex: $tx_hex,
        block_hash: $block_hash,
        block_height: 102,
        block_time: 1700061200,
        confirmations: 1,
        fee: 100000,
        inputs: [{
          tx_hash: $coinbase_txid,
          tx_pos: $coinbase_vout,
          address: $funding_address,
          value: 5000000000,
          script_pubkey: $coinbase_script
        }],
        outputs: [
          {tx_pos: 0, address: $a1, value: 100000000, script_pubkey: $decoded_tx.vout[0].scriptPubKey.hex},
          {tx_pos: 1, address: $a2, value: 200000000, script_pubkey: $decoded_tx.vout[1].scriptPubKey.hex},
          {tx_pos: 2, address: $a3, value: 300000000, script_pubkey: $decoded_tx.vout[2].scriptPubKey.hex},
          {tx_pos: 3, address: $a4, value: 400000000, script_pubkey: $decoded_tx.vout[3].scriptPubKey.hex},
          {tx_pos: 4, address: $a5, value: 3999900000, script_pubkey: $decoded_tx.vout[4].scriptPubKey.hex}
        ]
      },
      mempool: {
        txid: $mempool_txid,
        wtxid: $mempool_decoded.hash,
        hex: $mempool_hex,
        height: 0,
        fee: 100000,
        inputs: [{tx_hash: $txid, tx_pos: 0, address: $a1, value: 100000000}],
        outputs: [{
          tx_pos: 0,
          address: $a2,
          value: 99900000,
          script_pubkey: $mempool_decoded.vout[0].scriptPubKey.hex
        }]
      }
    }
  }' >"$fixture.tmp"
mv "$fixture.tmp" "$fixture"
jq . "$fixture"
