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
rf = np.clip(2.0 * (xs + 0.5) / W * 255.0, 0, 255)
gf = np.clip(2.0 * (H - (ys + 0.5)) / H * 255.0, 0, 255)
bf = np.full((H, W), 255.0)

# unclamped interior only (avoid where the ideal hits 0/255)
maskR = (rf > 2) & (rf < 253)
maskG = (gf > 2) & (gf < 253)

for name in ("chrome", "iphone"):
    B = by[name]["imgs"][1]
    print(f"\n{name}")
    for ch, f, mk, label in ((0, rf, maskR, "R"), (1, gf, maskG, "G"), (2, bf, None, "B")):
        obs = B[:, :, ch]
        if mk is None:
            print(f"  {label}: constant {obs.mean():.3f}"); continue
        best = None
        for b in np.arange(-0.50, 0.5001, 0.01):
            q = np.floor(f + 0.5 + b)
            mm = int(((q != obs) & mk).sum())
            if best is None or mm < best[0]:
                best = (mm, b)
        tot = int(mk.sum())
        print(f"  {label}: best bias {best[1]:+.2f} LSB -> {best[0]}/{tot} mismatches "
              f"({100*best[0]/tot:.3f}%)")
