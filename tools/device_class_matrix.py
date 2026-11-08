import base64, glob, io, json, os, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
from PIL import Image
import numpy as np

ROOT = r"C:\Users\oli\Desktop\canvasgen_rs"
DIR = os.path.join(ROOT, "captures_devices")

def short(pld):
    ua = pld.get("ua", "")
    gpu = (pld.get("gpu") or {}).get("renderer") or "?"
    osname = ("iOS" if "iPhone" in ua else "Android" if "Android" in ua
              else "Win" if "Windows" in ua else "?")
    br = "FF" if "Firefox" in ua else ("Safari" if "Safari" in ua and "Chrome" not in ua else "Chr")
    g = next((k for k in ("Apple", "Adreno", "Mali", "NVIDIA", "AMD", "Intel") if k.lower() in gpu.lower()), "?")
    scr = pld.get("screen") or {}
    return f"{osname}/{br}/{g}/{scr.get('w')}x{scr.get('h')}@{pld.get('dpr')}"

def load(p):
    d = json.load(open(p, encoding="utf-8")); pld = d["payload"]
    imgs = [np.asarray(Image.open(io.BytesIO(base64.b64decode(u.split(",",1)[1]))).convert("RGBA")).astype(np.int16) for u in pld["canvases"]]
    return {"file": os.path.basename(p), "pld": pld, "imgs": imgs, "tag": short(pld)}

caps = [load(p) for p in sorted(glob.glob(os.path.join(DIR, "*.json")))]
# keep the validated Chrome capture and all later captures
keep = [c for c in caps if c["file"] >= "20261108_112103"]
# keep the last iPhone repeat only
seen = set(); sel = []
for c in keep:
    key = (c["tag"], c["pld"].get("ts"))
    # dedupe by canvas signature
    import hashlib
    sig = hashlib.sha256(b"".join(a.tobytes() for a in c["imgs"])).hexdigest()
    if sig in seen: continue
    seen.add(sig); sel.append(c)
print("devices:")
for i, c in enumerate(sel):
    print(f"  {i}: {c['tag']}")

def white(a):
    rgb = a[:,:,:3].astype(float); al = a[:,:,3:4].astype(float)/255.0
    return np.rint(rgb*al + 255.0*(1-al)).astype(np.int16)

def m(a, b, i):
    A, B = a["imgs"][i], b["imgs"][i]
    if A.shape != B.shape: return None
    d = np.abs(white(A)-white(B)).max(axis=2)
    return float((d>0).mean()), float(d.mean()), int(d.max())

groups = {
    "iOS-iOS": [(i,j) for i in range(len(sel)) for j in range(i+1,len(sel))
                if sel[i]["tag"].startswith("iOS") and sel[j]["tag"].startswith("iOS")],
    "iOS-Android": [(i,j) for i in range(len(sel)) for j in range(i+1,len(sel))
                    if {sel[i]["tag"].split("/")[0], sel[j]["tag"].split("/")[0]} == {"iOS","Android"}],
    "FF-NVIDIA vs FF-AMD": [(i,j) for i in range(len(sel)) for j in range(i+1,len(sel))
                            if "FF/NVIDIA" in sel[i]["tag"] and "FF/AMD" in sel[j]["tag"]],
    "Chrome-NV vs FF-NV": [(i,j) for i in range(len(sel)) for j in range(i+1,len(sel))
                           if "Chr/NVIDIA" in sel[i]["tag"] and "FF/NVIDIA" in sel[j]["tag"]],
    "Chrome-NV vs iOS": [(i,j) for i in range(len(sel)) for j in range(i+1,len(sel))
                         if "Chr/NVIDIA" in sel[i]["tag"] and "iOS" in sel[j]["tag"]],
}
for name, pairs in groups.items():
    if not pairs: continue
    print(f"\n{name} ({len(pairs)} pairs)")
    for idx in range(5):
        vals = [m(sel[i], sel[j], idx) for i, j in pairs]
        vals = [v for v in vals if v]
        if not vals: continue
        fr = [v[0] for v in vals]; mn = [v[1] for v in vals]; mx = [v[2] for v in vals]
        print(f"  probe {idx}: frac {min(fr):.5f}..{max(fr):.5f} (med {np.median(fr):.5f})  "
              f"mean {min(mn):.3f}..{max(mn):.3f}  max {min(mx)}..{max(mx)}")
