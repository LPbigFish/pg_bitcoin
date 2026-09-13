#!/usr/bin/env python3
import hashlib, json, os, sys, base64, http.client

host = os.environ.get("BITCOIN_RPC_HOST", "127.0.0.1")
port = int(os.environ.get("BITCOIN_RPC_PORT", "18443"))
user = os.environ.get("BITCOIN_RPC_USER", "pg_bitcoin")
pw = os.environ.get("BITCOIN_RPC_PASSWORD", "pg_bitcoin")
fulcrum_url = os.environ.get("FULCRUM_URL", "tcp://127.0.0.1:60401")
state_dir = os.environ.get("REGTEST_STATE_DIR", os.path.join(os.path.dirname(__file__), "..", ".regtest"))
fixture_path = os.path.join(state_dir, "test-data.json")
_auth = "Basic " + base64.b64encode(f"{user}:{pw}".encode()).decode()

def rpc(method, params=None):
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params or []})
    c = http.client.HTTPConnection(host, port, timeout=30)
    c.request("POST", "/", body, {"Content-Type": "application/json", "Authorization": _auth})
    r = json.loads(c.getresponse().read())
    c.close()
    if r.get("error"): raise RuntimeError(f"RPC {method}: {r['error']}")
    return r["result"]

def derive(wif):
    desc = rpc("getdescriptorinfo", [f"wpkh({wif})"])["descriptor"]
    addr = rpc("deriveaddresses", [desc])[0]
    sh = hashlib.sha256(bytes.fromhex(rpc("validateaddress", [addr])["scriptPubKey"])).digest()[::-1].hex()
    return addr, sh

FUNDING_WIF = "cMahea7zqjxrtgAbB7LSGbcQUr1uX1ojuat9jZodMN87JcbXMTcA"
MINING_WIF = "cMahea7zqjxrtgAbB7LSGbcQUr1uX1ojuat9jZodMN87MbvFSpLf"
RECIPIENT_WIFS = [
    "cMahea7zqjxrtgAbB7LSGbcQUr1uX1ojuat9jZodMN87K7XCyj5v",
    "cMahea7zqjxrtgAbB7LSGbcQUr1uX1ojuat9jZodMN87KcLPVfXz",
    "cMahea7zqjxrtgAbB7LSGbcQUr1uX1ojuat9jZodMN87L7FgDCKE",
    "cMahea7zqjxrtgAbB7LSGbcQUr1uX1ojuat9jZodMN87Lc8ycuM4",
    "cMahea7zqjxrtgAbB7LSGbcQUr1uX1ojuat9jZodMN87M73ZA41f",
]
AMOUNTS = [1, 2, 3, 4, 39.999]
sat = lambda v: int(v * 100_000_000)

os.makedirs(state_dir, exist_ok=True)
open(os.path.join(state_dir, ".pg-bitcoin-regtest"), "a").close()

# Fixture cache
if os.path.exists(fixture_path):
    fx = json.load(open(fixture_path))
    if fx.get("schema_version") != 3:
        print("Outdated fixture; restart with --reset.", file=sys.stderr); sys.exit(1)
    try:
        rpc("getrawtransaction", [fx["transactions"]["confirmed"]["txid"], True,
             fx["transactions"]["confirmed"]["block_hash"]])
        rpc("getmempoolentry", [fx["transactions"]["mempool"]["txid"]])
        json.dump(fx, sys.stdout, indent=2)
        sys.exit(0)
    except Exception:
        print("Stale fixture; restart with --reset.", file=sys.stderr); sys.exit(1)

if rpc("getblockcount") != 0:
    print("Chain not empty; restart with --reset.", file=sys.stderr); sys.exit(1)

# Derive addresses
funding_addr, funding_sh = derive(FUNDING_WIF)
mining_addr, _ = derive(MINING_WIF)
recip = [derive(w) for w in RECIPIENT_WIFS]
recip_addrs, recip_shs = zip(*recip)

# Mine 101 blocks
rpc("generatetoaddress", [1, funding_addr])
rpc("generatetoaddress", [100, mining_addr])

# Coinbase UTXO from block 1
cb_txid = rpc("getblock", [rpc("getblockhash", [1])])["tx"][0]
cb_raw = rpc("getrawtransaction", [cb_txid, True])
cb_vouts = cb_raw["vout"]
cb_idx = next(i for i, o in enumerate(cb_vouts) if o["scriptPubKey"]["address"] == funding_addr)
cb_amount = cb_vouts[cb_idx]["value"]
cb_script = cb_vouts[cb_idx]["scriptPubKey"]["hex"]

# Confirmed split tx
inputs = [{"txid": cb_txid, "vout": cb_idx}]
outputs = [{a: v} for a, v in zip(recip_addrs, AMOUNTS)]
prevouts = [{"txid": cb_txid, "vout": cb_idx, "scriptPubKey": cb_script, "amount": cb_amount}]
signed = rpc("signrawtransactionwithkey", [
    rpc("createrawtransaction", [inputs, outputs, 0, False]),
    [FUNDING_WIF], prevouts])
assert signed["complete"]
tx_hex = signed["hex"]
decoded = rpc("decoderawtransaction", [tx_hex])
txid = rpc("sendrawtransaction", [tx_hex])

rpc("generatetoaddress", [1, mining_addr])
block_hash = rpc("getbestblockhash")

# Mempool tx
mp_prevouts = [{"txid": txid, "vout": 0, "scriptPubKey": decoded["vout"][0]["scriptPubKey"]["hex"], "amount": 1}]
mp_hex = rpc("signrawtransactionwithkey", [
    rpc("createrawtransaction", [[{"txid": txid, "vout": 0}], [{recip_addrs[1]: 0.999}], 0, False]),
    [RECIPIENT_WIFS[0]], mp_prevouts])["hex"]
mp_decoded = rpc("decoderawtransaction", [mp_hex])
mp_txid = rpc("sendrawtransaction", [mp_hex])

# Build fixture
S = sat
addr_list = [{
    "name": "funding", "address": funding_addr, "wif": FUNDING_WIF, "scripthash": funding_sh,
    "scripthash.get_history": [{"tx_hash": cb_txid, "height": 1}, {"tx_hash": txid, "height": 102}],
    "scripthash.get_mempool": [], "scripthash.get_balance": {"confirmed": 0, "unconfirmed": 0},
    "scripthash.listunspent": [], "spent_outpoint": {"tx_hash": cb_txid, "tx_pos": cb_idx},
}]
for k in range(1, 6):
    hist = [{"tx_hash": txid, "height": 102}]
    mp_hist, utxos = [], []
    if k <= 2:
        hist.append({"tx_hash": mp_txid, "height": 0})
        mp_hist = [{"tx_hash": mp_txid, "height": 0, "fee": 100000}]
    if k == 2:
        utxos = [{"tx_hash": txid, "tx_pos": 1, "height": 102, "value": 200_000_000,
                  "script_pubkey": decoded["vout"][1]["scriptPubKey"]["hex"]},
                 {"tx_hash": mp_txid, "tx_pos": 0, "height": 0, "value": 99_900_000,
                  "script_pubkey": mp_decoded["vout"][0]["scriptPubKey"]["hex"]}]
    elif k > 2:
        utxos = [{"tx_hash": txid, "tx_pos": k - 1, "height": 102, "value": S(AMOUNTS[k - 1]),
                  "script_pubkey": decoded["vout"][k - 1]["scriptPubKey"]["hex"]}]
    addr_list.append({
        "name": f"recipient_{k}", "address": recip_addrs[k - 1], "wif": RECIPIENT_WIFS[k - 1],
        "scripthash": recip_shs[k - 1], "scripthash.get_history": hist,
        "scripthash.get_mempool": mp_hist,
        "scripthash.get_balance": {"confirmed": S(AMOUNTS[k - 1]),
            "unconfirmed": -100_000_000 if k == 1 else (99_900_000 if k == 2 else 0)},
        "scripthash.listunspent": utxos,
    })

fixture = {
    "schema_version": 3, "network": "regtest", "fulcrum_url": fulcrum_url,
    "addresses": addr_list,
    "transactions": {
        "confirmed": {
            "txid": txid, "wtxid": decoded["hash"], "hex": tx_hex,
            "block_hash": block_hash, "block_height": 102, "block_time": 1700061200,
            "confirmations": 1, "fee": 100000,
            "inputs": [{"tx_hash": cb_txid, "tx_pos": cb_idx, "address": funding_addr,
                         "value": int(cb_amount * 100_000_000), "script_pubkey": cb_script}],
            "outputs": [{"tx_pos": i, "address": a, "value": S(v),
                          "script_pubkey": decoded["vout"][i]["scriptPubKey"]["hex"]}
                         for i, (a, v) in enumerate(zip(recip_addrs, AMOUNTS))],
        },
        "mempool": {
            "txid": mp_txid, "wtxid": mp_decoded["hash"], "hex": mp_hex,
            "height": 0, "fee": 100000,
            "inputs": [{"tx_hash": txid, "tx_pos": 0, "address": recip_addrs[0], "value": 100_000_000}],
            "outputs": [{"tx_pos": 0, "address": recip_addrs[1], "value": 99_900_000,
                          "script_pubkey": mp_decoded["vout"][0]["scriptPubKey"]["hex"]}],
        }
    }
}

with open(fixture_path + ".tmp", "w") as f:
    json.dump(fixture, f, indent=2); f.write("\n")
os.replace(fixture_path + ".tmp", fixture_path)
json.dump(fixture, sys.stdout, indent=2); print()
