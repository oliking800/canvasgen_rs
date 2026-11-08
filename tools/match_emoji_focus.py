from PIL import Image, ImageDraw, ImageFont
import numpy as np, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

FONT = r"C:\Windows\Fonts\seguiemj.ttf"
CAP = r"C:\Users\oli\Desktop\hcaptcha_research\captures_canvas\canvas_3_300x150.png"
im = Image.open(CAP).convert("RGBA")

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

boxes = {1: (64, 57, 77, 75), 3: (90, 57, 103, 75)}
cands = {
    1: ["🤪", "😜", "😝", "😀", "😃", "😆", "😄", "🤣"],
    3: ["👋", "👌", "🤙", "✋", "🖐️", "🫰", "👆", "👍", "🫶", "🙌"],
}
for i, box in boxes.items():
    crop = im.crop(box)
    out = []
    for ch in cands[i]:
        best = None
        for size in range(10, 19):
            g = render(ch, size)
            if g is None: continue
            g2 = g.resize(crop.size, Image.LANCZOS)
            mse = float(((to_white(crop) - to_white(g2))**2).mean())
            if best is None or mse < best[0]: best = (mse, size)
        out.append((best[0], ch, best[1]))
    out.sort()
    print(f"glyph {i}:")
    for mse, ch, size in out:
        print(f"   {ch!r} size={size} mse={mse:.1f}")
