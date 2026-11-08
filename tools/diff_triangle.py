from PIL import Image
import numpy as np, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

A = np.asarray(Image.open(r"C:\Users\oli\Desktop\canvasgen_rs\samples\device_lab_1.png").convert("RGBA")).astype(int)
B = np.asarray(Image.open(r"C:\Users\oli\Desktop\canvasgen_rs\samples\proof\dataurl_1.png").convert("RGBA")).astype(int)
d = np.abs(A - B).max(axis=2)
print("diff px", int((d > 0).sum()), "of", d.size, "max", int(d.max()), "mean", float(d.mean()))
for name, X in (("ours", A), ("ref", B)):
    m = X[:, :, :3].sum(axis=2) > 12
    ys, xs = np.where(m)
    print(name, "bbox x", xs.min(), xs.max(), "y", ys.min(), ys.max())
    for y in (37, 40, 60, 100, 126):
        row = np.where(m[y])[0]
        if len(row):
            print("   y", y, "x", row.min(), row.max())
# where do they differ
ys, xs = np.where(d > 0)
if len(xs):
    print("diff bbox x", xs.min(), xs.max(), "y", ys.min(), ys.max())
    # distribution by row
    import collections
    c = collections.Counter(ys)
    print("rows with most diffs:", c.most_common(8))
