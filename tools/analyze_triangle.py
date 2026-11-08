from PIL import Image
import numpy as np, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

a = np.asarray(Image.open(r"C:\Users\oli\Desktop\canvasgen_rs\samples\proof\dataurl_1.png").convert("RGBA"))
h, w, _ = a.shape
rgb = a[:, :, :3].astype(int)
mask = rgb.sum(axis=2) > 12  # not black
ys, xs = np.where(mask)
print("non-black bbox x", xs.min(), xs.max(), "y", ys.min(), ys.max(), "n", len(xs))
for y in range(ys.min(), ys.max() + 1, 8):
    row = np.where(mask[y])[0]
    if len(row):
        print(f"  y={y}: x {row.min()}..{row.max()}  colors r={rgb[y,row[len(row)//2],0]} g={rgb[y,row[len(row)//2],1]} b={rgb[y,row[len(row)//2],2]}")
# sample color formula at a few points
for (x, y) in [(150, 100), (40, 120), (260, 120), (150, 70)]:
    print((x, y), rgb[y, x], "formula", (2*x/w, 2*(h-y)/h, 1))
