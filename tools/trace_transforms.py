import json, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
t = json.load(open(r"C:\Users\oli\Desktop\canvasgen_rs\canvas_calls.json", encoding="utf-8"))
print("== transform / state ops in trace ==")
for i, e in enumerate(t):
    n = str(e["name"])
    if n in ("scale", "setTransform", "transform", "translate", "rotate", "save", "restore",
             "globalAlpha", "globalCompositeOperation", "textAlign", "textBaseline",
             "shadowBlur", "shadowColor", "shadowOffsetX", "shadowOffsetY", "lineWidth"):
        print(f"  {i:3d} {e['kind']:9s} {n:22s} {json.dumps(e['args'], ensure_ascii=False)[:120]}")
