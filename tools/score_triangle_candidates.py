import json, base64, io, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
from PIL import Image
import numpy as np

ROOT = r"C:\Users\oli\Desktop\canvasgen_rs"
ref_bytes = open(ROOT + r"\samples\proof\dataurl_1.png", "rb").read()
ref = np.asarray(Image.open(io.BytesIO(ref_bytes)).convert("RGB")).astype(int)
urls = json.load(open(ROOT + r"\tools\triangle_candidates.json"))
names = [
    "(149.5,37) (15,127) (269,127)",
    "(150,37.5) (14,127.5) (270,127.5)",
    "(149.5,37) (15,127.5) (269.5,127.5)",
    "(150,37.5) (14.5,127.5) (269.5,127.5)",
    "(149.5,37.5) (14,127.5) (270,127.5)",
    "(149.5,37) (14.5,127.5) (269.5,127.5)",
]
for i, u in enumerate(urls):
    b = base64.b64decode(u.split(",", 1)[1])
    im = np.asarray(Image.open(io.BytesIO(b)).convert("RGB")).astype(int)
    d = np.abs(im - ref).max(axis=2)
    print(f"{i}: diff {int((d>0).sum()):5d} mean {d.mean():6.2f} max {int(d.max()):3d}  {names[i]}")
    Image.open(io.BytesIO(b)).save(ROOT + rf"\samples\tri_cand_{i}.png")
