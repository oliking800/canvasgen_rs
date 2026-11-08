from PIL import Image
import os

def crop_scale(src, out, scale=8, pad=3):
    im = Image.open(src).convert("RGBA")
    px = im.load()
    w, h = im.size
    xs, ys = [], []
    for y in range(h):
        for x in range(w):
            r, g, b, a = px[x, y]
            if a > 8 and not (r > 245 and g > 245 and b > 245):
                xs.append(x); ys.append(y)
    if not xs:
        print("empty", src); return None
    x0, y0, x1, y1 = max(0, min(xs)-pad), max(0, min(ys)-pad), min(w, max(xs)+pad+1), min(h, max(ys)+pad+1)
    box = im.crop((x0, y0, x1, y1))
    box = box.resize((box.width*scale, box.height*scale), Image.NEAREST)
    bg = Image.new("RGBA", box.size, (255, 255, 255, 255))
    bg.alpha_composite(box)
    bg.convert("RGB").save(out)
    print(out, "crop", (x0, y0, x1, y1), "size", box.size)
    return box

a = crop_scale(r"C:\Users\oli\Desktop\hcaptcha_research\captures_canvas\canvas_3_300x150.png",
               r"C:\Users\oli\Desktop\hcaptcha_research\captures_canvas\emoji_capture_x8.png")
b = crop_scale(r"C:\Users\oli\Desktop\canvasgen_rs\samples\emoji_row_transparent.png",
               r"C:\Users\oli\Desktop\canvasgen_rs\samples\emoji_generated_x8.png")

if a and b:
    w = max(a.width, b.width)
    h = a.height + b.height + 12
    canvas = Image.new("RGB", (w, h), (255, 255, 255))
    canvas.paste(a.convert("RGB"), (0, 0))
    canvas.paste(b.convert("RGB"), (0, a.height + 12))
    canvas.save(r"C:\Users\oli\Desktop\canvasgen_rs\samples\emoji_capture_vs_generated_x8.png")
    print("saved side-by-side")
