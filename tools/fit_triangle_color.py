from PIL import Image
import numpy as np, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

B = np.asarray(Image.open(r"C:\Users\oli\Desktop\canvasgen_rs\samples\proof\dataurl_1.png").convert("RGB")).astype(int)
# fit r vs x, g vs y inside the triangle, away from edges
m = B.sum(axis=2) > 12
# erode mask a bit by requiring neighbors
mm = m.copy()
for dy in (-2,-1,1,2):
    for dx in (-2,-1,1,2):
        mm &= np.roll(np.roll(m, dy, axis=0), dx, axis=1)
ys, xs = np.where(mm)
print("samples", len(xs))
R = B[:, :, 0][mm]; G = B[:, :, 1][mm]; BL = B[:, :, 2][mm]
print("blue unique", np.unique(BL)[:10], "min/max", BL.min(), BL.max())
for name, V, C in (("R vs x", xs, R), ("G vs y", ys, G)):
    a, b = np.polyfit(V, C, 1)
    pred = a * V + b
    print(f"{name}: {a:.6f} * v + {b:.3f}  resid mean {np.abs(pred-C).mean():.3f} max {np.abs(pred-C).max():.1f}")
# check exact formula candidates
for expr, pred in (
    ("2x/w", 2*xs/300*255),
    ("2x/w*(255/256)?", 2*xs/300*255*255/256),
    ("x/w*2 round", np.round(2*xs/300*255)),
):
    print("R", expr, "mean abs", float(np.abs(pred - R).mean()))
for expr, pred in (
    ("2(h-y)/h", 2*(150-ys)/150*255),
    ("2(149-y)/149", 2*(149-ys)/149*255),
    ("2(150-y)/150*255*255/256", 2*(150-ys)/150*255*255/256),
):
    print("G", expr, "mean abs", float(np.abs(pred - G).mean()))
