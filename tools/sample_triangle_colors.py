from PIL import Image
import numpy as np, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

B = np.asarray(Image.open(r"C:\Users\oli\Desktop\canvasgen_rs\samples\proof\dataurl_1.png").convert("RGB")).astype(int)
pts = [(150,40),(150,50),(150,80),(150,110),(150,124),
       (30,120),(60,110),(90,100),(120,90),
       (240,120),(210,110),(180,100),(100,60),(200,60),(75,80),(225,80)]
for x, y in pts:
    print(f"({x:3d},{y:3d}) -> {tuple(B[y,x])}")
print("\nrow y=100 sample every 20 px:")
for x in range(60, 240, 20):
    print(x, tuple(B[100,x]))
print("\ncolumn x=150 sample every 10 px:")
for y in range(40, 128, 10):
    print(y, tuple(B[y,150]))
