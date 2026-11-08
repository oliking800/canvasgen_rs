from PIL import Image
import numpy as np, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

for fn in ("dataurl_0.png", "dataurl_2.png", "dataurl_4.png", "dataurl_3.png", "dataurl_1.png"):
    p = r"C:\Users\oli\Desktop\canvasgen_rs\samples\proof\\" + fn
    im = Image.open(p).convert("RGBA")
    a = np.asarray(im)
    al = a[:, :, 3]
    rgb = a[:, :, :3].astype(int)
    mx = rgb.max(axis=2); mn = rgb.min(axis=2)
    sat = (mx - mn) > 60
    def bb(mask, name):
        ys, xs = np.where(mask)
        if not len(xs):
            print(f"  {fn} {name}: empty"); return
        print(f"  {fn} {name}: x {xs.min()}..{xs.max()} y {ys.min()}..{ys.max()} n={len(xs)}")
    print(fn, im.size)
    bb(al > 8, "ink")
    bb((al > 8) & sat, "saturated")
