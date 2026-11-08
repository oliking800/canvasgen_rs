import base64, io, json, os, subprocess, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
from PIL import Image
import numpy as np

ROOT = r"C:\Users\oli\Desktop"
BIN = os.path.join(ROOT, "canvasgen_rs", "target", "release", "canvasgen.exe")
BASES = os.path.join(ROOT, "epic_hcaptcha_rs", "assets", "canvas_bases.json")
data = json.load(open(BASES, encoding="utf-8"))

def decode(url):
    b = base64.b64decode(url.split(",", 1)[1])
    return np.asarray(Image.open(io.BytesIO(b)).convert("RGBA")).astype(np.int16)

def white(a):
    rgb = a[:, :, :3].astype(float)
    al = a[:, :, 3:4].astype(float) / 255.0
    return np.rint(rgb * al + 255 * (1 - al)).astype(np.int16)

for cls in ("chrome", "iphone"):
    v = data["classes"][cls]["variants"][0]
    urls = v["urls"]
    for seed in (7, 12345):
        proc = subprocess.run([BIN, "vary", "--seed", str(seed)],
                              input="\n".join(urls), capture_output=True, text=True)
        assert proc.returncode == 0, proc.stderr
        out = [l for l in proc.stdout.splitlines() if l.strip()]
        print(f"\n{cls} seed {seed}: {len(out)} urls")
        for i, (a, b) in enumerate(zip(urls, out)):
            A, B = decode(a), decode(b)
            d = np.abs(white(A) - white(B)).max(axis=2)
            print(f"  probe {i}: {A.shape[1]}x{A.shape[0]} diff {float((d>0).mean()):.5f} "
                  f"mean {d.mean():.3f} max {int(d.max())}")
