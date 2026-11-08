import json, base64, io, sys, subprocess, os, tempfile, shutil
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
from PIL import Image
import numpy as np

ROOT = r"C:\Users\oli\Desktop\canvasgen_rs"
src = ROOT + r"\tools\triangle_opts.html"
tmpdir = tempfile.mkdtemp(prefix="chrome-opts-")
tmp = os.path.join(tmpdir, "t.html")
shutil.copyfile(src, tmp)
chrome = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
r = subprocess.run([chrome, "--headless=new", "--no-sandbox", "--hide-scrollbars",
                    "--virtual-time-budget=5000", "--dump-dom",
                    "file:///" + tmp.replace("\\", "/")],
                   capture_output=True, text=True, timeout=90000)
import re
m = re.search(r'data-out="([^"]+)"', r.stdout or "")
if not m:
    print("no output", (r.stdout or "")[:300]); sys.exit(1)
raw = m.group(1).replace("&quot;", '"').replace("&amp;", "&")
urls = json.loads(raw)
ref = np.asarray(Image.open(ROOT + r"\samples\proof\dataurl_1.png").convert("RGB")).astype(int)
names = ["aa+preserve", "no-aa", "alpha:false", "no-depth/stencil", "premult:false", "high-performance"]
for i, u in enumerate(urls):
    im = np.asarray(Image.open(io.BytesIO(base64.b64decode(u.split(",", 1)[1]))).convert("RGB")).astype(int)
    d = np.abs(im - ref).max(axis=2)
    print(f"{i}: diff {int((d>0).sum()):4d} mean {d.mean():.3f} max {int(d.max()):3d}  {names[i]}")
shutil.rmtree(tmpdir, ignore_errors=True)
