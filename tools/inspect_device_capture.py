import json, glob, os, base64, io, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
from PIL import Image

ROOT = r"C:\Users\oli\Desktop\canvasgen_rs"
f = sorted(glob.glob(os.path.join(ROOT, "captures_devices", "*.json")))[-1]
d = json.load(open(f, encoding="utf-8"))
p = d["payload"]
print("file", os.path.basename(f))
print("ua", p.get("ua")[:120])
print("dpr", p.get("dpr"), "gpu", (p.get("gpu") or {}).get("renderer"))
print("repeatIdentical", p.get("repeatIdentical"))
print("fonts", json.dumps(p.get("fonts"), ensure_ascii=False)[:300])
for i, u in enumerate(p.get("canvases") or []):
    if not u:
        print(i, "None"); continue
    b = base64.b64decode(u.split(",", 1)[1])
    im = Image.open(io.BytesIO(b)).convert("RGBA")
    print(i, im.size, "png", len(b), "sha", d["canvas_sha256"][i][:12])
    im.save(os.path.join(ROOT, "samples", f"device_lab_{i}.png"))
