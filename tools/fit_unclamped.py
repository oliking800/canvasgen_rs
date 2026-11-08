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

for name in ("chrome", "iphone"):
    B = by[name]["imgs"][1]
    print(f"\n{name}")
    # triangle interior with margin
    m = B.sum(axis=2) > 12
    mm = m.copy()
    for dy in range(-2, 3):
        for dx in range(-2, 3):
            mm &= np.roll(np.roll(m, dy, axis=0), dx, axis=1)
    ys, xs = np.where(mm)
    # unclamped bands
    for ch, cond, label in ((0, (xs < 140), "R (x<140)"),
                            (1, (ys > 60) & (ys < 120), "G (60<y<120)"),
                            (2, np.ones_like(ys, bool), "B")):
        sel = cond
        X = np.stack([xs[sel], ys[sel], np.ones(sel.sum())], 1)
        V = B[:, :, ch][mm][sel]
        c, *_ = np.linalg.lstsq(X, V, rcond=None)
        pred = X @ c
        print(f"  {label}: {c[0]:+.6f}x {c[1]:+.6f}y {c[2]:+.3f}  resid mean {np.abs(pred-V).mean():.3f} max {np.abs(pred-V).max():.2f}")
        for vn, (vx, vy) in {"apex": (150, 37.5), "left": (15, 127.5), "right": (270, 127.5)}.items():
            print(f"      {vn}: {c[0]*vx + c[1]*vy + c[2]:.2f}", end="")
        print()
