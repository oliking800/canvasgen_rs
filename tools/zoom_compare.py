from PIL import Image

ROOT = r"C:\Users\oli\Desktop\canvasgen_rs"
pairs = [("dataurl_0.png", "gen_0.png", "small", 10),
         ("dataurl_3.png", "gen_3.png", "text", 4)]

def white(im):
    bg = Image.new("RGBA", im.size, (255, 255, 255, 255))
    bg.alpha_composite(im.convert("RGBA"))
    return bg.convert("RGB")

for cap, gen, name, scale in pairs:
    a = white(Image.open(f"{ROOT}/samples/proof/{cap}"))
    b = white(Image.open(f"{ROOT}/samples/wasm_compare/{gen}"))
    a = a.resize((a.width * scale, a.height * scale), Image.NEAREST)
    b = b.resize((b.width * scale, b.height * scale), Image.NEAREST)
    w = max(a.width, b.width)
    sheet = Image.new("RGB", (w, a.height + b.height + 10), (230, 230, 230))
    sheet.paste(a, (0, 0))
    sheet.paste(b, (0, a.height + 10))
    out = f"{ROOT}/samples/wasm_compare/zoom_{name}.png"
    sheet.save(out)
    print(out, sheet.size)
