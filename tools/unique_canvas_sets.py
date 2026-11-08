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

# group identical sets
groups = {}
for name, urls in sets.items():
    h = hashlib.sha1("\n".join(urls).encode()).hexdigest()[:12]
    groups.setdefault(h, []).append(name)

print(f"{len(sets)} files -> {len(groups)} unique canvas sets")
for h, names in sorted(groups.items(), key=lambda kv: -len(kv[1])):
    print(f"  {h} x{len(names)}: {names[0]}")

uniq = {}
for name, urls in sets.items():
    h = hashlib.sha1("\n".join(urls).encode()).hexdigest()[:12]
    if h in uniq:
        continue
    arrs = []
    for u in urls:
        b = base64.b64decode(u.split(",", 1)[1])
        arrs.append(np.asarray(Image.open(io.BytesIO(b)).convert("RGBA")).astype(np.int16))
    uniq[h] = arrs

names = list(uniq)
print("\npairwise (unique sets):")
for i in range(5):
    print(f"\nprobe {i}")
    for a in range(len(names)):
        for b in range(a + 1, len(names)):
            A, B = uniq[names[a]][i], uniq[names[b]][i]
            if A.shape != B.shape:
                continue
            d = np.abs(A - B).max(axis=2)
            print(f"  {names[a][:8]} vs {names[b][:8]}: diff {float((d>0).mean()):.4f} mean {d.mean():.2f} max {int(d.max())}")
