import base64, glob, io, json, os, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
from PIL import Image
import numpy as np

ROOT = r"C:\Users\oli\Desktop\canvasgen_rs"
DIR = os.path.join(ROOT, "captures_devices")

def load(p):
    d = json.load(open(p, encoding="utf-8")); pld = d["payload"]
    imgs = [np.asarray(Image.open(io.BytesIO(base64.b64decode(u.split(",",1)[1]))).convert("RGB")).astype(float)
            for u in pld["canvases"]]
    return {"pld": pld, "imgs": imgs}

caps = [load(p) for p in sorted(glob.glob(os.path.join(DIR, "*.json")))]
by = {}
for c in caps:
    ua = c["pld"].get("ua","")
    if "HeadlessChrome" in ua: by["chrome"] = c
    elif "iPhone" in ua: by["iphone"] = c

W, H = 300, 150
xs = np.arange(W)[None, :].repeat(H, 0).astype(float)
ys = np.arange(H)[:, None].repeat(W, 1).astype(float)
rf = 2.0 * (xs + 0.5) / W * 255.0
gf = 2.0 * (H - (ys + 0.5)) / H * 255.0

for name in ("chrome", "iphone"):
    im = by[name]["imgs"][1]
    m = im.sum(axis=2) > 12
    # erode 2px so AA edges don't count
    mm = m.copy()
    for dy in range(-2, 3):
        for dx in range(-2, 3):
            mm &= np.roll(np.roll(m, dy, axis=0), dx, axis=1)
    print(f"\n{name}")
    for ch, f, label in ((0, rf, "R"), (1, gf, "G")):
        obs = im[:, :, ch]
        sel = mm & (f > 2) & (f < 253)
        best = None
        for b in np.arange(-0.50, 0.5001, 0.005):
            q = np.floor(f + 0.5 + b)
            n = int(((q != obs) & sel).sum())
            if best is None or n < best[0]:
                best = (n, b)
        tot = int(sel.sum())
        print(f"  {label}: best bias {best[1]:+.3f} LSB -> {best[0]}/{tot} "
              f"({100*best[0]/tot:.4f}%)")
