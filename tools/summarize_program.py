import json, sys, collections
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

d = json.load(open(r"C:\Users\oli\Desktop\canvasgen_rs\spec\canvas_program.json", encoding="utf-8"))
for c in d:
    if c["type"] != "2d":
        print(c["ctx_index"], c["type"], "no 2D ops"); continue
    cnt = collections.Counter(o["op"] for o in c["ops"])
    fonts = sorted({o["state"].get("font") for o in c["ops"] if isinstance(o.get("state"), dict) and o["state"].get("font")})
    lws = sorted({o["state"].get("lineWidth") for o in c["ops"] if isinstance(o.get("state"), dict) and o["state"].get("lineWidth") is not None})
    texts = [(o["op"], o["args"][0]) for o in c["ops"] if o["op"] in ("fillText", "strokeText")]
    print(f"ctx {c['ctx_index']} {c['width']}x{c['height']} url {c['data_url_index']} ops {len(c['ops'])}")
    print("   mix:", dict(cnt))
    print("   fonts:", fonts)
    print("   lineWidths:", lws)
    print("   texts:", texts)
