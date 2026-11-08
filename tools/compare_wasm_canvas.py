import json, sys, os, subprocess
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
from PIL import Image

ROOT = r"C:\Users\oli\Desktop\canvasgen_rs"
OUT = os.path.join(ROOT, "samples", "wasm_compare")
os.makedirs(OUT, exist_ok=True)

# generate the five probes through the CLI
exe = os.path.join(ROOT, "target", "release", "canvasgen.exe")
if not os.path.exists(exe):
    exe = os.path.join(ROOT, "target", "release", "canvasgen")
r = subprocess.run([exe, "urls", "--seed", "42"], capture_output=True, text=True,
                   cwd=ROOT, encoding="utf-8")
if r.returncode != 0:
    print("cli failed", r.returncode, r.stderr[:500]); sys.exit(1)
urls = json.loads(r.stdout)
if isinstance(urls, dict):
    urls = urls.get("data_urls") or urls.get("urls")
import base64
for i, u in enumerate(urls):
    b = base64.b64decode(u.split(",", 1)[1])
    open(os.path.join(OUT, f"gen_{i}.png"), "wb").write(b)

names = ["small", "webgl", "small-blob", "text", "abstract"]
for i in range(5):
    cap = Image.open(os.path.join(ROOT, "samples", "proof", f"dataurl_{i}.png")).convert("RGBA")
    gen = Image.open(os.path.join(OUT, f"gen_{i}.png")).convert("RGBA")
    w = max(cap.width, gen.width)
    h = cap.height + gen.height + 6
    sheet = Image.new("RGBA", (w, h), (255, 255, 255, 255))
    def on_white(im):
        bg = Image.new("RGBA", im.size, (255, 255, 255, 255))
        bg.alpha_composite(im)
        return bg
    sheet.alpha_composite(on_white(cap), (0, 0))
    sheet.alpha_composite(on_white(gen), (0, cap.height + 6))
    sheet.convert("RGB").save(os.path.join(OUT, f"compare_{i}_{names[i]}.png"))
    print(i, names[i], "captured", cap.size, "generated", gen.size)

