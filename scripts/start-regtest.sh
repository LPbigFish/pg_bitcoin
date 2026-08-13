#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
state_dir="$project_dir/.regtest"
marker="$state_dir/.pg-bitcoin-regtest"
compose=(docker compose --project-directory "$project_dir" -f "$project_dir/compose.yaml")

case "${1:-}" in
  "") reset=false ;;
  --reset) reset=true ;;
  *) printf 'Usage: %s [--reset]\n' "$0" >&2; exit 2 ;;
esac

if [[ -e "$state_dir" && ! -f "$marker" ]]; then
  printf 'Refusing to use unmarked directory: %s\n' "$state_dir" >&2
  exit 1
fi

if [[ "$reset" == "true" ]]; then
  "${compose[@]}" down --volumes --remove-orphans
  if [[ -e "$state_dir" ]]; then
    if [[ ! -f "$marker" ]]; then
      printf 'Refusing to remove unmarked directory: %s\n' "$state_dir" >&2
      exit 1
    fi
    rm -rf -- "$state_dir"
  fi
fi

mkdir -p "$state_dir"
touch "$marker"

"${compose[@]}" up --build --force-recreate --wait --wait-timeout 300
"${compose[@]}" run --rm --no-deps \
  -e FULCRUM_HOST=fulcrum \
  -e FULCRUM_PORT=50001 \
  fixture /usr/local/bin/check-regtest.sh

printf 'Bitcoin RPC: %s\n' "${BITCOIN_RPC_URL:-http://127.0.0.1:18443}"
printf 'Fulcrum: %s\n' "${FULCRUM_URL:-tcp://127.0.0.1:60401}"
printf 'Mempool UI: http://127.0.0.1:8080\n'
printf 'Fixture: %s/test-data.json\n' "$state_dir"
printf 'Stop: docker compose down\n'
