import json, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

P = r"C:\Users\oli\Desktop\hcaptcha_research\proof_emulation\chrome_proof_env.json"
d = json.load(open(P, encoding="utf-8"))
cap = d["cap"]

print("== ctxs ==")
for i, c in enumerate(cap["ctxs"]):
    print(f"  [{i}] {json.dumps(c, ensure_ascii=False)[:400]}")

print("\n== dataURLs ==")
for i, u in enumerate(cap["dataURLs"]):
    print(f"  [{i}] {json.dumps(u, ensure_ascii=False)[:200]}")

print("\n== entries ==")
for i, e in enumerate(cap["entries"]):
    print(f"  [{i}] {json.dumps(e, ensure_ascii=False)[:300]}")

print("\n== ops (137) ==")
for i, op in enumerate(cap["ops"]):
    print(f"  {i:3d} {json.dumps(op, ensure_ascii=False)[:220]}")
