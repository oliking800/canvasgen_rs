from PIL import Image
import numpy as np, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

im = Image.open(r"C:\Users\oli\Desktop\canvasgen_rs\samples\proof\dataurl_3.png").convert("RGBA")
a = np.asarray(im)
al = a[:, :, 3]
print("alpha max in ring around row:")
for y0, y1 in [(30, 58), (58, 60), (72, 95)]:
    reg = al[y0:y1, 30:140]
    print(f"  y {y0}..{y1}: max={reg.max()} nz={int((reg>0).sum())} sum={int(reg.sum())}")
# alpha profile column at x=75 (middle of row), rows 40..100
print("\nalpha at x=75:")
for y in range(40, 100):
    if al[y, 75]:
        print("  y", y, "a", al[y, 75], "rgb", tuple(a[y, 75, :3]))
