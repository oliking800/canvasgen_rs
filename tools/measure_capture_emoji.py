from PIL import Image
import numpy as np, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

im = Image.open(r"C:\Users\oli\Desktop\canvasgen_rs\samples\proof\dataurl_3.png").convert("RGBA")
a = np.asarray(im)
h, w, _ = a.shape
rgb = a[:, :, :3].astype(int)
alpha = a[:, :, 3]

def bbox(mask, name):
    ys, xs = np.where(mask)
    if len(xs) == 0:
        print(name, "empty"); return
    print(f"{name}: x {xs.min()}..{xs.max()} (w={xs.max()-xs.min()+1}), y {ys.min()}..{ys.max()} (h={ys.max()-ys.min()+1})")

# region of interest
roi = np.zeros((h, w), bool); roi[45:90, 40:130] = True
bbox(roi & (alpha > 8), "any ink")
# saturated emoji pixels
mx = rgb.max(axis=2); mn = rgb.min(axis=2)
sat = (mx - mn) > 90
bbox(roi & (alpha > 8) & sat, "saturated (emoji core)")
# very dark pixels (text fill, if black)
bbox(roi & (alpha > 8) & (mx < 40), "dark")

# print vertical profile of saturated pixels in the row
sub = (roi & (alpha > 8) & sat)
for y in range(50, 80):
    xs = np.where(sub[y])[0]
    if len(xs):
        print(y, "x", xs.min(), "..", xs.max(), "n", len(xs))
