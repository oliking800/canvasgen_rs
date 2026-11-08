import json, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
t = json.load(open(r"C:\Users\oli\Desktop\canvasgen_rs\canvas_calls.json", encoding="utf-8"))
print("total", len(t))
print("save count", sum(1 for e in t if e["kind"] == "call" and e["name"] == "save"))
print("restore count", sum(1 for e in t if e["kind"] == "call" and e["name"] == "restore"))
print("\nsave/restore events:")
for i, e in enumerate(t):
    if e["kind"] == "call" and e["name"] in ("save", "restore"):
        print(" ", i, e["name"])
print("\nall font sets:")
for i, e in enumerate(t):
    if e["kind"] == "set" and e["name"] == "font":
        print(f"  {i:3d} {e['args'][0][:70]}")
print("\nall fillStyle sets (non-gradient):")
for i, e in enumerate(t):
    if e["kind"] == "set" and e["name"] == "fillStyle":
        v = e["args"][0]
        print(f"  {i:3d} {json.dumps(v, ensure_ascii=False)[:70]}")
