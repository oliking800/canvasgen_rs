from PIL import Image
import numpy as np, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

im = Image.open(r"C:\Users\oli\Desktop\canvasgen_rs\samples\proof\dataurl_2.png").convert("RGBA")
a = np.asarray(im)
print("alpha min/max", a[:,:,3].min(), a[:,:,3].max(), "opaque frac", (a[:,:,3]==255).mean())
from collections import Counter
c = Counter(map(tuple, a[:, :, :3].reshape(-1, 3)))
print("top colors (rgb):")
for col, n in c.most_common(12):
    print("  ", col, n)

# upscale for viewing
im.resize((im.width*10, im.height*10), Image.NEAREST).save(
    r"C:\Users\oli\Desktop\canvasgen_rs\samples\wasm_compare\cap2_x10.png")
