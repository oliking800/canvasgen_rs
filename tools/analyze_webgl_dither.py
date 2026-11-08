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
    return {"pld": pld, "imgs": imgs}

caps = [load(p) for p in sorted(glob.glob(os.path.join(DIR, "*.json")))]
by = {}
for c in caps:
    ua = c["pld"].get("ua","")
    if "HeadlessChrome" in ua: by["chrome"] = c
    elif "iPhone" in ua: by["iphone"] = c

A = by["chrome"]["imgs"][1]; B = by["iphone"]["imgs"][1]
d = np.abs(A.astype(int) - B.astype(int)).max(axis=2)
mask = (d > 0)
ys, xs = np.where(mask)
print("n", len(xs), "of", mask.size, "pct", 100*len(xs)/mask.size)
# interior block
y0, x0 = 80, 100
print("\nmask block (y 80..91, x 100..131):")
for y in range(y0, y0+12):
    print("  " + "".join("#" if mask[y,x] else "." for x in range(x0, x0+32)))
print("\nby x mod 4:", [int(mask[:, x::4].sum()) for x in range(4)])
print("by y mod 4:", [int(mask[y::4, :].sum()) for y in range(4)])
print("by x mod 8:", [int(mask[:, x::8].sum()) for x in range(8)])
print("by y mod 8:", [int(mask[y::8, :].sum()) for y in range(8)])
# run lengths along rows
runs = []
for y in range(mask.shape[0]):
    row = mask[y]
    n = 0
    for v in row:
        if v: n += 1
        elif n: runs.append(n); n = 0
    if n: runs.append(n)
import collections
print("row run lengths:", collections.Counter(runs).most_common(8))
# does the delta depend on channel/value?
print("delta signs (A-B) for R,G,B at differing px:")
for ch, name in enumerate("RGB"):
    dd = A[:,:,ch][mask].astype(int) - B[:,:,ch][mask].astype(int)
    print(" ", name, collections.Counter(dd.tolist()).most_common(5))
