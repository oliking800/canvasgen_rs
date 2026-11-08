import json, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

t = json.load(open(r"C:\Users\oli\Desktop\canvasgen_rs\canvas_calls.json", encoding="utf-8"))
print("trace calls 0..20:")
k = 0
for i, e in enumerate(t):
    if e["kind"] == "call" and e["name"] not in ("toDataURL", "getContext"):
        print(f"  idx{i:3d} call#{k:3d} {e['name']:20s} {json.dumps(e['args'], ensure_ascii=False)[:80]}")
        k += 1
        if k > 20:
            break
print("\nfillRect(0,0,2,2) occurrences:")
for i, e in enumerate(t):
    if e["kind"] == "call" and e["name"] == "fillRect" and e["args"][:4] == [0, 0, 2, 2]:
        print("  idx", i, e["args"])
