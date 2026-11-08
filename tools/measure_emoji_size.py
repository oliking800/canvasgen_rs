from PIL import Image, ImageDraw, ImageFont
import numpy as np, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

FONT = r"C:\Windows\Fonts\seguiemj.ttf"
text = "\u2764\ufe0f\U0001f92a\U0001f389\U0001f44b"
for size in (6.68896, 8, 10, 13, 15, 20):
    # PIL needs int-ish; use float via truetype size float? try
    try:
        f = ImageFont.truetype(FONT, size)
    except Exception as e:
        print(size, "err", e); continue
    mode = f.getmask(text, mode="L")
    print(f"size={size}: mask bbox={mode.size}, length={mode.getbbox()}")

# exact pixel width via draw at high res
for size in (6.68896, 13.37792):
    f = ImageFont.truetype(FONT, size)
    img = Image.new("L", (400, 100), 0)
    d = ImageDraw.Draw(img)
    d.text((10, 40), text, font=f, fill=255, embedded_color=False)
    a = np.asarray(img)
    xs = np.where(a.max(axis=0) > 8)[0]
    ys = np.where(a.max(axis=1) > 8)[0]
    print(f"size={size}: ink x {xs.min()}-{xs.max()} w={xs.max()-xs.min()+1} h={ys.max()-ys.min()+1}")
