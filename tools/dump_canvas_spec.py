import json, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

t = json.load(open(r"C:\Users\oli\Desktop\canvasgen_rs\canvas_calls.json", encoding="utf-8"))

def j(x):
    return json.dumps(x, ensure_ascii=False)

# gradient ids -> coords from gradNew / createRadialGradient events
grad = {}
pending = None
for e in t:
    if e["kind"] == "call" and e["name"] in ("createLinearGradient", "createRadialGradient"):
        pending = {"kind": e["name"], "coords": list(e["args"])}
    elif e["kind"] == "gradNew":
        g = dict(pending or {"kind": "?", "coords": None})
        g["stops"] = []
        grad[e["name"]] = g
    elif e["kind"] == "gradStop":
        if e["name"] in grad:
            grad[e["name"]]["stops"].append(list(e["args"]))

print("== canvases (getContext) ==")
for e in t:
    if e["kind"] == "getContext":
        print(f"  event {t.index(e):3d} type={e['args'][0]!r} size={e['args'][1]}x{e['args'][2]}")

print("\n== toDataURL order ==")
for i, e in enumerate(t):
    if e["kind"] == "toDataURL":
        print(f"  event {i:3d} idx={e['args'][0]} len={e['args'][1]} size={e['args'][2]}x{e['args'][3]}")

row = next(i for i, e in enumerate(t)
           if e["kind"] == "call" and e["name"] == "fillText"
           and isinstance(e["args"][0], str) and e["args"][0] == "\u2764\ufe0f\U0001f92a\U0001f389\U0001f44b")

# replay state with save/restore
state = {}
stack = []
for e in t[:row]:
    if e["kind"] == "set":
        state[e["name"]] = e["args"][0]
    elif e["kind"] == "call" and e["name"] == "save":
        stack.append(dict(state))
    elif e["kind"] == "call" and e["name"] == "restore":
        if stack:
            state = stack.pop()

print(f"\n== emoji row (event {row}) ==")
for k, v in state.items():
    if isinstance(v, dict) and "__id" in v:
        g = grad.get(v["__id"], {})
        print(f"  {k} = {g.get('kind')}{j(g.get('coords'))} stops={j(g.get('stops'))}")
    else:
        print(f"  {k} = {j(v)}")

print("\n== all 2D draws in order (text + rect + gradient usage) ==")
cur = {}
for i, e in enumerate(t):
    if e["kind"] == "set":
        cur[e["name"]] = e["args"][0]
    if e["kind"] == "call" and e["name"] in ("fillText", "strokeText", "fillRect", "strokeRect"):
        fs = cur.get("fillStyle")
        if isinstance(fs, dict) and "__id" in fs:
            g = grad.get(fs["__id"], {})
            fs = f"{g.get('kind')}{j(g.get('coords'))}{j(g.get('stops'))}"
        print(f"  {i:3d} {e['name']:10s} {j(e['args'])[:90]:92s} font={cur.get('font','')[:34]!r} fill={j(fs)[:90]}")
