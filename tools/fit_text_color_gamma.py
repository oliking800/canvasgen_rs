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

for idx in (0, 3, 2):
    A = white(by["chrome"]["imgs"][idx]); B = white(by["firefox"]["imgs"][idx])
    m = (A < 250).any(axis=2) & (A > 5).any(axis=2)
    print(f"\nprobe {idx}: pixels {int(m.sum())}")
    for ch, name in enumerate("RGB"):
        a = A[:,:,ch][m]; b = B[:,:,ch][m]
        if len(a) < 10: continue
        # fit A = 255*(B/255)^g + c
        best = None
        for g in np.arange(0.90, 1.101, 0.005):
            pred = 255.0*np.power(np.clip(b/255.0,0,1), g)
            # allow additive offset
            c0 = np.mean(a - pred)
            mse = np.mean((a - (pred+c0))**2)
            if best is None or mse < best[0]: best = (mse, g, c0)
        print(f"  {name}: gamma {best[1]:.3f} offset {best[2]:+.2f} rmse {np.sqrt(best[0]):.2f}")
