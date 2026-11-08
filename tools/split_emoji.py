from PIL import Image

def split(src, prefix, x0, x1, y0, y1, n=4, scale=18):
    im = Image.open(src).convert("RGBA")
    w = (x1 - x0) / n
    for i in range(n):
        a = int(round(x0 + i * w)); b = int(round(x0 + (i + 1) * w))
        crop = im.crop((a, y0, b, y1))
        crop = crop.resize((crop.width * scale, crop.height * scale), Image.NEAREST)
        bg = Image.new("RGBA", crop.size, (255, 255, 255, 255)); bg.alpha_composite(crop)
        out = f"{prefix}_{i}.png"
        bg.convert("RGB").save(out)
        print(out, (a, b))

split(r"C:\Users\oli\Desktop\hcaptcha_research\captures_canvas\canvas_3_300x150.png",
      r"C:\Users\oli\Desktop\canvasgen_rs\samples\capture_emoji", 51, 103, 57, 75)
split(r"C:\Users\oli\Desktop\canvasgen_rs\samples\emoji_row_transparent.png",
      r"C:\Users\oli\Desktop\canvasgen_rs\samples\generated_emoji", 51, 101, 57, 75)

# side-by-side per position
for i in range(4):
    a = Image.open(rf"C:\Users\oli\Desktop\canvasgen_rs\samples\capture_emoji_{i}.png")
    b = Image.open(rf"C:\Users\oli\Desktop\canvasgen_rs\samples\generated_emoji_{i}.png")
    w = max(a.width, b.width); h = a.height + b.height + 8
    c = Image.new("RGB", (w, h), (255, 255, 255))
    c.paste(a, (0, 0)); c.paste(b, (0, a.height + 8))
    c.save(rf"C:\Users\oli\Desktop\canvasgen_rs\samples\pair_{i}.png")
print("pairs saved")
