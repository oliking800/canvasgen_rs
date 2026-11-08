import json, glob, os, sys, base64, io
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
from PIL import Image
import numpy as np

ROOT = r"C:\Users\oli\Desktop\hcaptcha_research"
files = glob.glob(os.path.join(ROOT, "**", "*.json"), recursive=True)

def urls_of(d):
    if not isinstance(d, dict):
        return None
    for key in ("canvasDataURLs",):
        v = d.get(key)
        if isinstance(v, list) and len(v) == 5 and all(isinstance(x, str) for x in v):
            return v
    cap = d.get("cap")
    if isinstance(cap, dict):
        v = cap.get("dataURLs")
        if isinstance(v, list) and len(v) == 5:
            out = []
            for u in v:
                if isinstance(u, str):
                    out.append(u)
                elif isinstance(u, dict) and "full" in u:
                    out.append(u["full"])
            if len(out) == 5:
                return out
    return None

sets = {}
for p in sorted(set(files)):
    try:
        d = json.load(open(p, encoding="utf-8"))
    except Exception:
        continue
    u = urls_of(d)
    if u:
        sets[os.path.relpath(p, ROOT)] = u

print("capture sets:", len(sets))
imgs = {}
for name, urls in sets.items():
    arrs = []
    for i, u in enumerate(urls):
        try:
            b = base64.b64decode(u.split(",", 1)[1])
            im = Image.open(io.BytesIO(b)).convert("RGBA")
            arrs.append(np.asarray(im).astype(np.int16))
        except Exception as e:
            arrs.append(None)
    imgs[name] = arrs

for i in range(5):
    group = [(n, a[i]) for n, a in imgs.items() if a[i] is not None]
    sizes = {}
    for n, a in group:
        sizes.setdefault(a.shape[:2], []).append(n)
    print(f"\nprobe {i}: {len(group)} captures, sizes {[(k, len(v)) for k, v in sizes.items()]}")
    # pairwise within the dominant size bucket
    size, names = max(sizes.items(), key=lambda kv: len(kv[1]))
    sel = [a for n, a in group if a.shape[:2] == size]
    if len(sel) < 2:
        continue
    fracs, means, p99s, maxs = [], [], [], []
    for x in range(len(sel)):
        for y in range(x + 1, len(sel)):
            d = np.abs(sel[x] - sel[y]).max(axis=2)
            fracs.append(float((d > 0).mean()))
            means.append(float(d.mean()))
            p99s.append(float(np.percentile(d, 99)))
            maxs.append(int(d.max()))
    print(f"  same-size pairs ({size}): n={len(fracs)}")
    print(f"    differing-pixel fraction: min {min(fracs):.4f} median {np.median(fracs):.4f} max {max(fracs):.4f}")
    print(f"    mean |d|: min {min(means):.2f} median {np.median(means):.2f} max {max(means):.2f}")
    print(f"    p99 |d|: min {min(p99s):.0f} median {np.median(p99s):.0f} max {max(p99s):.0f}")
    print(f"    max |d|: {sorted(set(maxs))[:8]}")
