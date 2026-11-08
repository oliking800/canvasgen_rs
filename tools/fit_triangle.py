from PIL import Image
import numpy as np, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

B = np.asarray(Image.open(r"C:\Users\oli\Desktop\canvasgen_rs\samples\proof\dataurl_1.png").convert("RGB")).astype(int)
m = B.sum(axis=2) > 12
ys, xs = np.where(m)
rows = []
for y in range(ys.min(), ys.max() + 1):
    row = np.where(m[y])[0]
    if len(row) > 2:
        rows.append((y, row.min() - 0.5, row.max() + 0.5))

# fit lines for left and right edges (exclude top 3 and bottom 3 rows)
body = rows[4:-3]
Y = np.array([r[0] + 0.5 for r in body], dtype=float)
XL = np.array([r[1] for r in body])
XR = np.array([r[2] for r in body])
for name, X in (("left", XL), ("right", XR)):
    a, b = np.polyfit(Y, X, 1)
    print(f"{name}: x = {a:.5f}*y + {b:.3f}")

aL, bL = np.polyfit(Y, XL, 1)
aR, bR = np.polyfit(Y, XR, 1)
y_apex = (bR - bL) / (aL - aR)
x_apex = aL * y_apex + bL
print("apex", x_apex, y_apex)
for y in (37.5, 127.5):
    print("y", y, "left", aL * y + bL, "right", aR * y + bR)
