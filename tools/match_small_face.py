from PIL import Image, ImageDraw, ImageFont
import numpy as np, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

FONT = r"C:\Windows\Fonts\seguiemj.ttf"
CAP = r"C:\Users\oli\Desktop\hcaptcha_research\captures_canvas\canvas_0_20x20.png"
im = Image.open(CAP).convert("RGBA")
px = im.load()
xs = [x for y in range(im.height) for x in range(im.width) if px[x, y][3] > 8]
ys = [y for y in range(im.height) for x in range(im.width) if px[x, y][3] > 8]
box = (min(xs), min(ys), max(xs)+1, max(ys)+1)
crop = im.crop(box)
print("crop", box, crop.size)

def to_white(img):
    bg = Image.new("RGBA", img.size, (255,255,255,255)); bg.alpha_composite(img)
    return np.asarray(bg.convert("RGB"), dtype=np.float32)

def render(ch, size):
    f = ImageFont.truetype(FONT, size)
    c = Image.new("RGBA", (size*3, size*3), (0,0,0,0))
    d = ImageDraw.Draw(c)
    d.text((size, size//2), ch, font=f, embedded_color=True)
    b = c.getbbox()
    return c.crop(b) if b else None

for ch in ["😀","😃","😄","😁","🤪","😜","😝","😊","🙂","😆"]:
    best = None
    for size in range(12, 26):
        g = render(ch, size)
        if g is None: continue
        g2 = g.resize(crop.size, Image.LANCZOS)
        mse = float(((to_white(crop) - to_white(g2))**2).mean())
        if best is None or mse < best[0]: best = (mse, size)
    print(f"{ch!r} best size={best[1]} mse={best[0]:.1f}")
