"""Compare every capture in captures_devices/ and report the real-device
variance envelope (differing-pixel fraction, mean/p99/max |d|) per canvas.
"""
import base64, glob, io, json, os, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
from PIL import Image
import numpy as np

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DIR = os.path.join(ROOT, "captures_devices")

def load():
    out = []
    for p in sorted(glob.glob(os.path.join(DIR, "*.json"))):
        try:
            d = json.load(open(p, encoding="utf-8"))
        except Exception:
            continue
        pld = d.get("payload") or {}
        imgs = []
        for u in pld.get("canvases") or []:
            if not u:
                imgs.append(None); continue
            try:
                imgs.append(np.asarray(Image.open(io.BytesIO(base64.b64decode(u.split(",", 1)[1]))).convert("RGBA")).astype(np.int16))
            except Exception:
                imgs.append(None)
        out.append({"file": os.path.basename(p), "ua": pld.get("ua", ""),
                    "gpu": (pld.get("gpu") or {}).get("renderer"),
                    "dpr": pld.get("dpr"), "imgs": imgs})
    return out

def on_white(a):
    if a is None: return None
    rgb = a[:, :, :3].astype(float)
    al = a[:, :, 3:4].astype(float) / 255.0
    return np.rint(rgb * al + 255.0 * (1 - al)).astype(np.int16)

caps = load()
print(f"{len(caps)} captures")
for c in caps:
    print(f"  {c['file']}: dpr={c['dpr']} gpu={c['gpu']}")

for idx in range(5):
    rows = [(c["file"], on_white(c["imgs"][idx])) for c in caps if c["imgs"][idx] is not None]
    if len(rows) < 2:
        continue
    print(f"\nprobe {idx} ({rows[0][1].shape[1]}x{rows[0][1].shape[0]})")
    for i in range(len(rows)):
        for j in range(i + 1, len(rows)):
            A, B = rows[i][1], rows[j][1]
            if A.shape != B.shape:
                continue
            d = np.abs(A - B).max(axis=2)
            print(f"  {rows[i][0][:12]} vs {rows[j][0][:12]}: "
                  f"diff {float((d>0).mean()):.5f} mean {d.mean():.3f} "
                  f"p99 {int(np.percentile(d,99))} max {int(d.max())}")

# per-device signature = sha of each canvas, for quick duplicates
print("\nsignatures")
for c in caps:
    import hashlib
    sig = [hashlib.sha256(a.tobytes()).hexdigest()[:10] if a is not None else "-" for a in c["imgs"]]
    print(f"  {c['file']}: {sig}")
