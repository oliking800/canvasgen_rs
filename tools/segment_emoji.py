from PIL import Image

def glyphs(src, prefix, scale=16, pad=2):
    im = Image.open(src).convert("RGBA")
    px = im.load(); w, h = im.size
    cols = []
    for x in range(w):
        on = False
        for y in range(h):
            r, g, b, a = px[x, y]
            if a > 8 and not (r > 245 and g > 245 and b > 245):
                on = True; break
        cols.append(on)
    # group columns
    groups, start = [], None
    for x, on in enumerate(cols):
        if on and start is None: start = x
        if not on and start is not None: groups.append((start, x)); start = None
    if start is not None: groups.append((start, w))
    # merge groups with small gaps
    merged = []
    for g in groups:
        if merged and g[0] - merged[-1][1] <= 2: merged[-1] = (merged[-1][0], g[1])
        else: merged.append(list(g))
    print(prefix, "glyph column groups", merged)
    for i, (x0, x1) in enumerate(merged):
        ys = [y for y in range(h) for x in range(x0, x1)
              if px[x, y][3] > 8 and not (px[x, y][0] > 245 and px[x, y][1] > 245 and px[x, y][2] > 245)]
        y0, y1 = max(0, min(ys)-pad), min(h, max(ys)+pad+1)
        crop = im.crop((max(0, x0-pad), y0, min(w, x1+pad), y1))
        crop = crop.resize((crop.width*scale, crop.height*scale), Image.NEAREST)
        bg = Image.new("RGBA", crop.size, (255, 255, 255, 255)); bg.alpha_composite(crop)
        out = f"{prefix}_glyph{i}.png"
        bg.convert("RGB").save(out)
        print("  ", out, "x", (x0, x1), "w", x1-x0, "h", y1-y0)

glyphs(r"C:\Users\oli\Desktop\hcaptcha_research\captures_canvas\canvas_3_300x150.png",
       r"C:\Users\oli\Desktop\canvasgen_rs\samples\cap")
glyphs(r"C:\Users\oli\Desktop\canvasgen_rs\samples\emoji_row_transparent.png",
       r"C:\Users\oli\Desktop\canvasgen_rs\samples\gen")
