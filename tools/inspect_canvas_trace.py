import json, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

t = json.load(open(r"C:\Users\oli\Desktop\canvasgen_rs\canvas_calls.json", encoding="utf-8"))
print("events", len(t))

# find the emoji row call
idx = None
for i, e in enumerate(t):
    if e["kind"] == "call" and e["name"] == "fillText" and isinstance(e["args"][0], str) and e["args"][0] == "❤️🤪🎉👋":
        idx = i
        break
print("emoji row call index", idx)

def show(a, b, title):
    print("\n==", title)
    for i in range(max(0, a), min(len(t), b)):
        e = t[i]
        args = e["args"]
        s = json.dumps(args, ensure_ascii=False)
        print(f"  {i:3d} {e['kind']:8s} {e['name']:22s} {s[:160]}")

if idx is not None:
    show(idx - 35, idx + 8, "around emoji row")
    # the 96x48 abstract starts after the row
    show(idx + 8, idx + 40, "after row (abstract 96x48)")

# print all top-level property sets in order
print("\n== property set order ==")
for i, e in enumerate(t):
    if e["kind"] == "set":
        print(f"  {i:3d} {e['name']} = {json.dumps(e['args'][0], ensure_ascii=False)[:120]}")
