import base64, glob, io, json, os, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
from PIL import Image
import numpy as np

ROOT = r"C:\Users\oli\Desktop\canvasgen_rs"
DIR = os.path.join(ROOT, "captures_devices")

def load(p):
    d = json.load(open(p, encoding="utf-8")); pld = d["payload"]
    imgs = [np.asarray(Image.open(io.BytesIO(base64.b64decode(u.split(",",1)[1]))).convert("RGB")).astype(int)
            for u in pld["canvases"]]
    return {"pld": pld, "imgs": imgs}

caps = [load(p) for p in sorted(glob.glob(os.path.join(DIR, "*.json")))]
by = {}
for c in caps:
    ua = c["pld"].get("ua","")
    if "HeadlessChrome" in ua: by["chrome"] = c
    elif "iPhone" in ua: by["iphone"] = c

W, H = 300, 150
xs = np.arange(W)[None, :].repeat(H, 0)
ys = np.arange(H)[:, None].repeat(W, 1)
rf = (2.0 * (xs + 0.5) / W) * 255.0
gf = (2.0 * (ys + 0.5) / H) * 255.0
bf = np.full((H, W), 255.0)

def quant(v, mode):
    v = np.clip(v, 0, 255)
    if mode == "round": return np.floor(v + 0.5).astype(int)
    if mode == "floor": return np.floor(v).astype(int)
    if mode == "ceil": return np.ceil(v).astype(int)
    if mode == "even": return np.rint(v).astype(int)
    raise ValueError(mode)

ink = None
for name in ("chrome", "iphone"):
    im = by[name]["imgs"][1]
    print(f"\n{name}:")
    m = im.sum(axis=2) > 12
    for mode in ("round", "floor", "ceil", "even"):
        mism = 0
        for ch, f in enumerate((rf, gf, bf)):
            q = quant(f, mode)
            mm = (q != im[:, :, ch]) & m
            mism += int(mm.sum())
        print(f"  {mode:6s}: mismatched channel-pixels {mism}")
    # bias fit for R and G
    for ch, f, nm in ((0, rf, "R"), (1, gf, "G")):
        obs = im[:, :, ch]
        # mean signed error vs round
        err = (obs - rf)[m]
        print(f"  {nm} mean(obs - float) = {err.mean():+.4f}  std {err.std():.4f}")
