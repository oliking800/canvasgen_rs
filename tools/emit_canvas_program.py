import json, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

SRC = r"C:\Users\oli\Desktop\canvasgen_rs\canvas_calls.json"
OUT = r"C:\Users\oli\Desktop\canvasgen_rs\spec\canvas_program.json"
t = json.load(open(SRC, encoding="utf-8"))

# Resolve gradient objects -> {kind, coords, stops}
grad = {}
pending = None
for e in t:
    if e["kind"] == "gradNew":
        g = dict(pending or {"kind": "?", "coords": None})
        g["stops"] = []
        grad[e["name"]] = g
    elif e["kind"] == "gradStop":
        if e["name"] in grad:
            grad[e["name"]]["stops"].append(list(e["args"]))
    elif e["kind"] == "call" and e["name"] in ("createLinearGradient", "createRadialGradient"):
        pending = {"kind": e["name"], "coords": list(e["args"])}

def resolve(v):
    if isinstance(v, dict) and "__id" in v:
        g = grad.get(v["__id"], {})
        return {"type": "gradient", "kind": g.get("kind"), "coords": g.get("coords"),
                "stops": g.get("stops")}
    return v

state = {}
stack = []
canvases = []
cur = None
canvas_index = -1

for i, e in enumerate(t):
    k, n = e["kind"], e["name"]
    if k == "getContext":
        canvas_index += 1
        cur = {"ctx_index": canvas_index, "type": e["args"][0],
               "width": e["args"][1], "height": e["args"][2], "ops": []}
        canvases.append(cur)
        # CanvasRenderingContext2D defaults (as recorded in the browser env).
        state = {"font": "10px sans-serif", "fillStyle": "#000000",
                 "strokeStyle": "#000000", "lineWidth": 1,
                 "textAlign": "start", "textBaseline": "alphabetic"}
        stack = []
        continue
    if k == "set":
        state[n] = resolve(e["args"][0])
        continue
    if k == "call" and n == "save":
        stack.append(dict(state)); continue
    if k == "call" and n == "restore":
        if stack: state = stack.pop()
        continue
    if k == "toDataURL":
        cur["data_url_index"] = e["args"][0]
        cur["data_url_len"] = e["args"][1]
        cur["width"], cur["height"] = e["args"][2], e["args"][3]
        continue
    if k == "call" and n in ("getImageData",):
        cur["ops"].append({"op": n, "args": e["args"]})
        continue
    if k == "call" and n in ("createLinearGradient", "createRadialGradient"):
        continue
    if k == "call":
        op = {"op": n, "args": [resolve(a) for a in e["args"]],
              "state": {kk: resolve(vv) for kk, vv in state.items()}}
        cur["ops"].append(op)

# split trailing duplicate: index 5 is a repeat of index 0 (same 20x20)
out = []
for c in canvases:
    d = dict(c)
    d["ops"] = [o for o in c["ops"] if o["op"] not in ("clearRect",)]
    out.append(d)

import os
os.makedirs(os.path.dirname(OUT), exist_ok=True)
json.dump(out, open(OUT, "w", encoding="utf-8"), ensure_ascii=False, indent=1)
print("wrote", OUT, "canvases", len(out))
for c in out:
    print(f"  ctx{c['ctx_index']} {c['type']} {c['width']}x{c['height']} -> url {c.get('data_url_index')} ops {len(c['ops'])}")
    for o in c["ops"]:
        st = o.get("state") or {}
        fs = st.get("fillStyle")
        fs_s = (f"{fs['kind']}{fs['coords']} {fs['stops']}" if isinstance(fs, dict) else str(fs))
        print(f"    {o['op']:18s} {json.dumps(o['args'], ensure_ascii=False)[:60]:62s} font={str(st.get('font'))[:28]!r} fill={fs_s[:60]}")
