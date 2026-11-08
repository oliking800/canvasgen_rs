import json, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

t = json.load(open(r"C:\Users\oli\Desktop\canvasgen_rs\canvas_calls.json", encoding="utf-8"))
row = next(i for i, e in enumerate(t) if e["kind"] == "call" and e["name"] == "fillText"
           and isinstance(e["args"][0], str) and e["args"][0] == "❤️🤪🎉👋")
print("emoji row at event", row)

# state as of the row
state = {}
font_at = None
for i in range(row):
    e = t[i]
    if e["kind"] == "set":
        state[e["name"]] = e["args"][0]
    if e["name"] == "font":
        font_at = (i, e["args"][0])
    if e["kind"] == "gradStop":
        state.setdefault("_grad", {}).setdefault(e["name"], []).append(e["args"])

print("\nstate before row:")
for k, v in state.items():
    print(f"  {k}: {json.dumps(v, ensure_ascii=False)}")

print("\nfont set at event", font_at[0], "len", len(font_at[1]))
print(font_at[1])

# gradient construction in order
print("\ngradients:")
for i, e in enumerate(t):
    if e["name"] in ("createLinearGradient", "createRadialGradient", "gradStop"):
        print(f"  {i:3d} {e['name']:22s} {json.dumps(e['args'], ensure_ascii=False)}")

print("\nlast 25 events before row:")
for i in range(max(0, row - 25), row):
    e = t[i]
    print(f"  {i:3d} {e['kind']:8s} {e['name']:22s} {json.dumps(e['args'], ensure_ascii=False)[:200]}")
