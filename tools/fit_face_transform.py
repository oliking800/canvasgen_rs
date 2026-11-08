import base64, glob, io, json, os, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
from PIL import Image
import numpy as np

ROOT = r"C:\Users\oli\Desktop\canvasgen_rs"
DIR = os.path.join(ROOT, "captures_devices")

def load(p):
    d = json.load(open(p, encoding="utf-8")); pld = d["payload"]
    imgs = [np.asarray(Image.open(io.BytesIO(base64.b64decode(u.split(",",1)[1]))).convert("RGBA")).astype(np.float32) for u in pld["canvases"]]
    return {"pld": pld, "imgs": imgs}

caps = [load(p) for p in sorted(glob.glob(os.path.join(DIR, "*.json")))]
by = {}
for c in caps:
    ua = c["pld"].get("ua","")
    if "HeadlessChrome" in ua: by["chrome"] = c
    elif "Firefox" in ua and "NVIDIA" in (c["pld"].get("gpu",{}).get("renderer","")): by["firefox"] = c

def white(a):
    rgb = a[:,:,:3]; al = a[:,:,3:4]/255.0
    return np.rint(rgb*al + 255.0*(1-al)).astype(np.float32)

for name in ("chrome","firefox"):
    a = by[name]["imgs"][0]
    w = white(a)
    sat = (w.max(axis=2) - w.min(axis=2)) > 40
    ys, xs = np.where(sat)
    print(name, "sat bbox", xs.min(), xs.max(), ys.min(), ys.max(),
          "centroid", xs.mean().round(2), ys.mean().round(2))
    al = a[:,:,3]
    ys, xs = np.where(al > 8)
    print("   ink bbox", xs.min(), xs.max(), ys.min(), ys.max(), "n", len(xs))

A = white(by["chrome"]["imgs"][0])
B = white(by["firefox"]["imgs"][0])
# brute-force similarity: scale B about its center, translate, compare to A
best = None
for scale in np.arange(0.90, 1.101, 0.01):
    for dx in np.arange(-1.5, 1.51, 0.25):
        for dy in np.arange(-1.5, 1.51, 0.25):
            im = Image.fromarray(B.astype(np.uint8))
            nw, nh = int(round(20*scale)), int(round(20*scale))
            im2 = im.resize((nw, nh), Image.BICUBIC)
            canvas = Image.new("RGB", (20, 20), (255,255,255))
            canvas.paste(im2, (int(round(dx)), int(round(dy))))
            C = np.asarray(canvas).astype(np.float32)
            mse = float(((A-C)**2).mean())
            if best is None or mse < best[0]:
                best = (mse, scale, dx, dy)
print("best B->A transform: mse %.2f scale %.2f dx %.2f dy %.2f" % best)
