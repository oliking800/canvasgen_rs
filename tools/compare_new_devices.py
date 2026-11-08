import base64, glob, io, json, os, sys, hashlib
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
from PIL import Image
import numpy as np

ROOT = r"C:\Users\oli\Desktop\canvasgen_rs"
DIR = os.path.join(ROOT, "captures_devices")

def load(p):
    d = json.load(open(p, encoding="utf-8"))
    pld = d.get("payload") or {}
    imgs = []
    for u in pld.get("canvases") or []:
        if not u:
            imgs.append(None); continue
        imgs.append(np.asarray(Image.open(io.BytesIO(base64.b64decode(u.split(",", 1)[1]))).convert("RGBA")).astype(np.int16))
    return {"file": os.path.basename(p), "ts": d.get("received_iso"), "pld": pld, "imgs": imgs}

caps = [load(p) for p in sorted(glob.glob(os.path.join(DIR, "*.json")))]
# keep the three relevant ones: chrome reference, iphone (last), firefox
by = {}
for c in caps:
    ua = c["pld"].get("ua", "")
    if "HeadlessChrome" in ua:
        by["chrome"] = c
    elif "Firefox" in ua:
        by["firefox"] = c
    elif "iPhone" in ua:
        by.setdefault("iphone", []).append(c)

print("== device info ==")
for name in ("chrome", "firefox"):
    c = by.get(name)
    if not c: continue
    p = c["pld"]
    print(f"\n{name}: {c['file']}")
    print("  ua", p.get("ua"))
    print("  dpr", p.get("dpr"), "screen", p.get("screen"), "win", p.get("window"))
    print("  gpu", json.dumps(p.get("gpu"), ensure_ascii=False))
    print("  repeatIdentical", p.get("repeatIdentical"))
    print("  fonts", json.dumps(p.get("fonts"), ensure_ascii=False))

for c in by.get("iphone", []):
    p = c["pld"]
    print(f"\niphone: {c['file']}")
    print("  ua", p.get("ua"))
    print("  dpr", p.get("dpr"), "screen", p.get("screen"), "win", p.get("window"))
    print("  gpu", json.dumps(p.get("gpu"), ensure_ascii=False))
    print("  repeatIdentical", p.get("repeatIdentical"))
    print("  fonts", json.dumps(p.get("fonts"), ensure_ascii=False))

def white(a):
    rgb = a[:, :, :3].astype(float); al = a[:, :, 3:4].astype(float) / 255.0
    return np.rint(rgb * al + 255.0 * (1 - al)).astype(np.int16)

def compare(a, b, label):
    print(f"\n{label}")
    for i in range(5):
        A, B = a["imgs"][i], b["imgs"][i]
        if A is None or B is None or A.shape != B.shape:
            print(f"  probe {i}: shape {None if A is None else A.shape} vs {None if B is None else B.shape}")
            continue
        d = np.abs(white(A) - white(B)).max(axis=2)
        print(f"  probe {i}: diff {float((d>0).mean()):.5f} mean {d.mean():7.3f} "
              f"p99 {int(np.percentile(d,99)):3d} max {int(d.max()):3d}")

iph = by.get("iphone", [])
if len(iph) >= 2:
    compare(iph[-2], iph[-1], "iphone repeat (same device)")
if "chrome" in by and iph:
    compare(by["chrome"], iph[-1], "chrome vs iphone")
if "chrome" in by and "firefox" in by:
    compare(by["chrome"], by["firefox"], "chrome vs firefox (same PC)")
if "firefox" in by and iph:
    compare(by["firefox"], iph[-1], "firefox vs iphone")
