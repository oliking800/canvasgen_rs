import base64, glob, io, json, os, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
from PIL import Image
import numpy as np

ROOT = r"C:\Users\oli\Desktop\canvasgen_rs"
DIR = os.path.join(ROOT, "captures_devices")

def load(p):
    d = json.load(open(p, encoding="utf-8")); pld = d["payload"]
    imgs = [np.asarray(Image.open(io.BytesIO(base64.b64decode(u.split(",",1)[1]))).convert("RGBA")).astype(np.int16)
            for u in pld["canvases"]]
    return {"file": os.path.basename(p), "pld": pld, "imgs": imgs}

caps = [load(p) for p in sorted(glob.glob(os.path.join(DIR, "*.json")))]
by = {}
for c in caps:
    ua = c["pld"].get("ua","")
    if "HeadlessChrome" in ua: by["chrome"] = c
    elif "Firefox" in ua: by["firefox"] = c
    elif "iPhone" in ua: by["iphone"] = c

def white(a):
    rgb = a[:,:,:3].astype(float); al = a[:,:,3:4].astype(float)/255.0
    return np.rint(rgb*al + 255.0*(1-al)).astype(np.int16)

def edge_mask(alpha, k=2):
    m = alpha > 0
    # pixels within k of an alpha boundary
    from scipy import ndimage  # may be absent
    return None

def diff_profile(a, b, idx, label):
    A, B = white(a["imgs"][idx]), white(b["imgs"][idx])
    d = np.abs(A-B).max(axis=2)
    ys, xs = np.where(d > 0)
    print(f"\n{label} probe {idx}: n={len(xs)} ({d.size} px)")
    if not len(xs): return
    vals = d[ys, xs]
    print("  delta histogram:", {int(v): int((vals==v).sum()) for v in sorted(set(vals.tolist()))[:10]})
    # coverage of diff relative to ink (alpha>0) and edges
    alA = a["imgs"][idx][:,:,3]
    ink = alA > 0
    print(f"  diff on ink: {float((ink[ys,xs]).mean()):.3f}; ink frac {ink.mean():.3f}")
    # is it 1-LSB mostly?
    for t in (1,2,4,8,16,32,64,128):
        print(f"   <= {t:3d}: {float((vals<=t).mean()):.3f}", end="")
    print()

pairs = [("chrome","iphone","chrome vs iphone"), ("chrome","firefox","chrome vs firefox")]
for a,b,label in pairs:
    if a in by and b in by:
        for idx in range(5):
            diff_profile(by[a], by[b], idx, label)
