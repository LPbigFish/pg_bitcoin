#!/usr/bin/env python3
import json, os, sys, socket

host = os.environ.get("FULCRUM_HOST", "127.0.0.1")
port = int(os.environ.get("FULCRUM_PORT", "60401"))
fixture = os.path.join(os.environ.get("REGTEST_STATE_DIR", os.path.join(os.path.dirname(__file__), "..", ".regtest")), "test-data.json")

def rpc(method, params):
    s = socket.create_connection((host, port), timeout=10)
    s.sendall(json.dumps({"id": 1, "method": method, "params": params}).encode() + b"\n")
    f = s.makefile()
    resp = json.loads(f.readline())
    f.close(); s.close()
    if resp.get("error"):
        print(json.dumps(resp), file=sys.stderr); sys.exit(1)
    return resp

def verify(label, got, want, cmp):
    if not cmp(got, want):
        print(f"{label} mismatch:\n{json.dumps(got)}", file=sys.stderr); sys.exit(1)

fx = json.load(open(fixture))
if not fx.get("addresses"):
    print(f"Fixture has no addresses: {fixture}", file=sys.stderr); sys.exit(1)

for entry in fx["addresses"]:
    name, sh = entry["name"], entry["scripthash"]

    r = rpc("blockchain.scripthash.get_history", [sh])
    got = [{k: v for k, v in x.items() if k in ("tx_hash", "height")} for x in r["result"]]
    verify(f"{name} get_history", got, entry["scripthash.get_history"],
           lambda g, w: sorted(g, key=lambda x: (x["height"], x["tx_hash"])) == sorted(w, key=lambda x: (x["height"], x["tx_hash"])))

    r = rpc("blockchain.scripthash.get_mempool", [sh])
    verify(f"{name} get_mempool", r["result"], entry["scripthash.get_mempool"],
           lambda g, w: sorted(g, key=lambda x: x["tx_hash"]) == sorted(w, key=lambda x: x["tx_hash"]))

    r = rpc("blockchain.scripthash.get_balance", [sh])
    verify(f"{name} get_balance", r["result"], entry["scripthash.get_balance"],
           lambda g, w: g == w)

    r = rpc("blockchain.scripthash.listunspent", [sh])
    verify(f"{name} listunspent", r["result"], entry["scripthash.listunspent"],
           lambda g, w: sorted(g, key=lambda x: (x["height"], x["tx_hash"], x["tx_pos"])) ==
                        sorted([{k: v for k, v in u.items() if k in ("tx_hash", "tx_pos", "height", "value")} for u in w],
                               key=lambda x: (x["height"], x["tx_hash"], x["tx_pos"])))

print(f"Verified four scripthash methods for {len(fx['addresses'])} addresses.")
