from PIL import Image, ImageDraw, ImageFont
import numpy as np
import os, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

FONT = r"C:\Windows\Fonts\seguiemj.ttf"
CAP = r"C:\Users\oli\Desktop\hcaptcha_research\captures_canvas\canvas_3_300x150.png"
im = Image.open(CAP).convert("RGBA")
px = im.load()

# glyph boxes from the capture (x0,x1,y0,y1)
boxes = [(51, 64, 57, 75), (64, 77, 57, 75), (77, 90, 57, 75), (90, 103, 57, 75)]

def to_white(img):
    bg = Image.new("RGBA", img.size, (255, 255, 255, 255))
    bg.alpha_composite(img)
    return np.asarray(bg.convert("RGB"), dtype=np.float32)

def render(ch, size):
    f = ImageFont.truetype(FONT, size)
    canvas = Image.new("RGBA", (size * 3, size * 3), (0, 0, 0, 0))
    d = ImageDraw.Draw(canvas)
    try:
        d.text((size, size // 2), ch, font=f, embedded_color=True)
    except TypeError:
        d.text((size, size // 2), ch, font=f, fill=(0, 0, 0, 255))
    bbox = canvas.getbbox()
    if not bbox:
        return None
    return canvas.crop(bbox)

def score(a, b):
    # resize b to a's size, white background, MSE
    b2 = b.resize(a.size, Image.LANCZOS)
    A = to_white(a); B = to_white(b2)
    return float(((A - B) ** 2).mean())

cands = {
    0: ["❤️", "❤", "🖤", "💔", "♥️"],
    1: ["😂", "😄", "😁", "😜", "😝", "😆", "🤣", "😊", "😃"],
    2: ["🎤", "🎙️", "📞", "🔦"],
    3: ["👏", "🙌", "🤝", "👋", "🫰", "🤟", "✌️", "👍", "🤙", "👌", "👐", "🙏", "🫶", "🤲", "👐", "🤞"],
}

for i, (x0, x1, y0, y1) in enumerate(boxes):
    crop = im.crop((x0, y0, x1, y1))
    results = []
    for ch in cands[i]:
        best = None
        for size in range(10, 19):
            g = render(ch, size)
            if g is None:
                continue
            s = score(crop, g)
            if best is None or s < best[0]:
                best = (s, ch, size)
        if best:
            results.append(best)
    results.sort()
    print(f"glyph {i}:")
    for s, ch, size in results[:5]:
        print(f"   {ch!r} size={size} mse={s:.1f}")
