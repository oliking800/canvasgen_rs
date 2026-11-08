from PIL import Image
import numpy as np, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

B = np.asarray(Image.open(r"C:\Users\oli\Desktop\canvasgen_rs\samples\proof\dataurl_1.png").convert("RGB")).astype(int)
m = B.sum(axis=2) > 12
mm = m.copy()
for dy in (-2,-1,1,2):
    for dx in (-2,-1,1,2):
        mm &= np.roll(np.roll(m, dy, axis=0), dx, axis=1)
ys, xs = np.where(mm)
X = np.stack([xs, ys, np.ones_like(xs)], axis=1).astype(float)
print("samples", len(xs))
coef = {}
for ch, name in enumerate("RGB"):
    V = B[:, :, ch][mm].astype(float)
    c, *_ = np.linalg.lstsq(X, V, rcond=None)
    pred = X @ c
    coef[name] = c
    print(f"{name}: {c[0]:+.6f}*x {c[1]:+.6f}*y {c[2]:+.3f}   resid mean {np.abs(pred-V).mean():.3f} max {np.abs(pred-V).max():.1f}")
# vertex values using fitted planes
verts = {"apex": (149.5, 37.0), "left": (14.0, 127.5), "right": (270.0, 127.5)}
for n, (x, y) in verts.items():
    print(n, {ch: round(float(coef[ch][0]*x + coef[ch][1]*y + coef[ch][2]), 2) for ch in "RGB"})
