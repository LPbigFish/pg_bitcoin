#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
fixture="${REGTEST_STATE_DIR:-$project_dir/.regtest}/test-data.json"
host="${FULCRUM_HOST:-127.0.0.1}"
port="${FULCRUM_PORT:-60401}"

rpc() {
  local request="$1" response
  exec 3<>"/dev/tcp/$host/$port"
  printf '%s\n' "$request" >&3
  if ! IFS= read -r -t 10 response <&3; then
    printf 'Fulcrum did not respond at %s:%s.\n' "$host" "$port" >&2
    exit 1
  fi
  exec 3>&- 3<&-
  if ! jq -e '.error == null' <<<"$response" >/dev/null; then
    printf '%s\n' "$response" >&2
    exit 1
  fi
  printf '%s\n' "$response"
}

verify() {
  local label="$1" response="$2" expected="$3" filter="$4"
  if ! jq -e --argjson expected "$expected" "$filter" <<<"$response" >/dev/null; then
    printf '%s mismatch:\n%s\n' "$label" "$response" >&2
    exit 1
  fi
}

if ! jq -e '.addresses | length > 0' "$fixture" >/dev/null; then
  printf 'Fixture has no addresses: %s\n' "$fixture" >&2
  exit 1
fi

mapfile -t addresses < <(jq -c '.addresses[]' "$fixture")
for address_data in "${addresses[@]}"; do
  name="$(jq -r '.name' <<<"$address_data")"
  scripthash="$(jq -r '.scripthash' <<<"$address_data")"

  response="$(rpc "$(jq -cn --arg scripthash "$scripthash" '{id: 1, method: "blockchain.scripthash.get_history", params: [$scripthash]}')")"
  expected="$(jq -c '."scripthash.get_history"' <<<"$address_data")"
  verify "$name scripthash.get_history" "$response" "$expected" '([.result[] | {tx_hash, height}] | sort_by(.height, .tx_hash)) == ($expected | sort_by(.height, .tx_hash))'

  expected="$(jq -c '."scripthash.get_mempool"' <<<"$address_data")"
  response="$(rpc "$(jq -cn --arg scripthash "$scripthash" '{id: 2, method: "blockchain.scripthash.get_mempool", params: [$scripthash]}')")"
  verify "$name scripthash.get_mempool" "$response" "$expected" '([.result[] | {tx_hash, height, fee}] | sort_by(.tx_hash)) == ($expected | sort_by(.tx_hash))'

  response="$(rpc "$(jq -cn --arg scripthash "$scripthash" '{id: 3, method: "blockchain.scripthash.get_balance", params: [$scripthash]}')")"
  expected="$(jq -c '."scripthash.get_balance"' <<<"$address_data")"
  verify "$name scripthash.get_balance" "$response" "$expected" '.result == $expected'

  response="$(rpc "$(jq -cn --arg scripthash "$scripthash" '{id: 4, method: "blockchain.scripthash.listunspent", params: [$scripthash]}')")"
  expected="$(jq -c '."scripthash.listunspent"' <<<"$address_data")"
  verify "$name scripthash.listunspent" "$response" "$expected" '([.result[] | {tx_hash, tx_pos, height, value}] | sort_by(.height, .tx_hash, .tx_pos)) == ([$expected[] | {tx_hash, tx_pos, height, value}] | sort_by(.height, .tx_hash, .tx_pos))'
done

printf 'Verified four scripthash methods for %s addresses.\n' "${#addresses[@]}"
