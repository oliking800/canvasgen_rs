import base64, glob, io, json, os, sys, hashlib
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
from PIL import Image
import numpy as np

ROOT = r"C:\Users\oli\Desktop\canvasgen_rs"
DIR = os.path.join(ROOT, "captures_devices")

def label(pld):
    ua = pld.get("ua", "")
    gpu = (pld.get("gpu") or {}).get("renderer") or "?"
    if "iPhone" in ua: osname = "iOS"
    elif "Android" in ua: osname = "Android"
    elif "Windows" in ua: osname = "Windows"
    elif "Mac OS X" in ua or "Macintosh" in ua: osname = "macOS"
    elif "Linux" in ua: osname = "Linux"
    else: osname = "?"
    if "Firefox" in ua: br = "Firefox"
    elif "Edg/" in ua: br = "Edge"
    elif "Chrome/" in ua or "CriOS" in ua: br = "Chrome"
    elif "Safari" in ua: br = "Safari"
    else: br = "?"
    # short gpu tag
    g = gpu
    for key in ("Apple", "Adreno", "Mali", "NVIDIA", "AMD", "Intel", "PowerVR", "Xclipse"):
        if key.lower() in gpu.lower():
            g = key; break
    else:
        g = gpu[:24]
    return f"{osname}/{br}/{g}"

def load(p):
    d = json.load(open(p, encoding="utf-8")); pld = d["payload"]
    imgs = []
    for u in pld.get("canvases") or []:
        if not u: imgs.append(None); continue
        imgs.append(np.asarray(Image.open(io.BytesIO(base64.b64decode(u.split(",",1)[1]))).convert("RGBA")).astype(np.int16))
    sig = hashlib.sha256(b"".join(b"" if a is None else a.tobytes() for a in imgs)).hexdigest()[:10]
    return {"file": os.path.basename(p), "ts": d.get("received_iso"), "pld": pld, "imgs": imgs,
            "sig": sig, "tag": label(pld)}

caps = [load(p) for p in sorted(glob.glob(os.path.join(DIR, "*.json")))]
print(f"{len(caps)} captures\n")
for c in caps:
    p = c["pld"]
    print(f"{c['ts']}  {c['tag']:28s} dpr={p.get('dpr')} "
          f"screen={p.get('screen',{}).get('w')}x{p.get('screen',{}).get('h')} "
          f"repeat={all(p.get('repeatIdentical') or [False])} sig={c['sig']}")

def white(a):
    rgb = a[:,:,:3].astype(float); al = a[:,:,3:4].astype(float)/255.0
    return np.rint(rgb*al + 255.0*(1-al)).astype(np.int16)

def metrics(a, b, idx):
    A, B = a["imgs"][idx], b["imgs"][idx]
    if A is None or B is None or A.shape != B.shape:
        return None
    d = np.abs(white(A)-white(B)).max(axis=2)
    return float((d>0).mean()), float(d.mean()), int(np.percentile(d,99)), int(d.max())

# group captures by device tag and signature
print("\n== unique device renders ==")
uniq = {}
for c in caps:
    uniq.setdefault(c["sig"], []).append(c)
for sig, group in uniq.items():
    print(f"  {sig} x{len(group)}: {group[0]['tag']}  ({', '.join(g['ts'][11:] for g in group)})")
