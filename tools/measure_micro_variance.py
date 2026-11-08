import json, glob, os, sys, base64, io, hashlib
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
from PIL import Image
import numpy as np

ROOT = r"C:\Users\oli\Desktop\hcaptcha_research"
files = glob.glob(os.path.join(ROOT, "**", "*.json"), recursive=True)

def urls_of(d):
    if not isinstance(d, dict):
        return None
    v = d.get("canvasDataURLs")
    if isinstance(v, list) and len(v) == 5 and all(isinstance(x, str) for x in v):
        return v
    cap = d.get("cap")
    if isinstance(cap, dict):
        v = cap.get("dataURLs")
        if isinstance(v, list) and len(v) == 5:
            out = [u if isinstance(u, str) else u.get("full") for u in v]
            if all(out):
                return out
    return None

uniq = {}
for p in sorted(set(files)):
    try:
        d = json.load(open(p, encoding="utf-8"))
    except Exception:
        continue
    u = urls_of(d)
    if not u:
        continue
    h = hashlib.sha1("\n".join(u).encode()).hexdigest()[:8]
    if h in uniq:
        continue
    arrs = []
    for s in u:
        b = base64.b64decode(s.split(",", 1)[1])
        im = Image.open(io.BytesIO(b)).convert("RGBA")
        bg = Image.new("RGBA", im.size, (255, 255, 255, 255))
        bg.alpha_composite(im)
        arrs.append(np.asarray(bg.convert("RGB")).astype(np.int16))
    uniq[h] = arrs

names = list(uniq)
print("sets", names)
for i in range(5):
    print(f"\nprobe {i}")
    rows = []
    for a in range(len(names)):
        for b in range(a + 1, len(names)):
            A, B = uniq[names[a]][i], uniq[names[b]][i]
            if A.shape != B.shape:
                continue
            d = np.abs(A - B).max(axis=2)
            rows.append((float((d > 0).mean()), float(d.mean()), int(np.percentile(d, 99)), int(d.max()),
                         names[a], names[b]))
    rows.sort()
    for frac, mean, p99, mx, a, b in rows:
        tag = "near" if frac < 0.10 else ("class" if frac > 0.4 else "mid")
        print(f"  {tag:5s} {a} vs {b}: diff {frac:.4f} mean {mean:.2f} p99 {p99} max {mx}")
