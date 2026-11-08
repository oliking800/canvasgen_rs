import json, base64, sys, os
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
from PIL import Image

P = r"C:\Users\oli\Desktop\hcaptcha_research\proof_emulation\chrome_proof_env.json"
d = json.load(open(P, encoding="utf-8"))
cap = d["cap"]
out = r"C:\Users\oli\Desktop\canvasgen_rs\samples\proof"
os.makedirs(out, exist_ok=True)

for i, u in enumerate(cap["dataURLs"]):
    full = u["full"] if isinstance(u, dict) else u
    b = base64.b64decode(full.split(",", 1)[1])
    fn = os.path.join(out, f"dataurl_{i}.png")
    open(fn, "wb").write(b)
    im = Image.open(fn)
    print(i, "len", len(full), "bytes", len(b), "img", im.size, im.mode)

# analyze row colors in dataurl_3
im = Image.open(os.path.join(out, "dataurl_3.png")).convert("RGB")
px = im.load()
# bounding box of emoji row region
cols = {}
for y in range(50, 80):
    for x in range(40, 120):
        c = px[x, y]
        cols[c] = cols.get(c, 0) + 1
print("\ntop colors in emoji region:")
for c, n in sorted(cols.items(), key=lambda kv: -kv[1])[:12]:
    print("  ", c, n)
