import sys, re, glob, os
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

row = "❤️🤪🎉👋"
units = []
for ch in row:
    o = ord(ch)
    if o > 0xFFFF:
        o -= 0x10000
        units += [0xD800 + (o >> 10), 0xDC00 + (o & 0x3FF)]
    else:
        units.append(o)

needles = {
    "utf8": row.encode("utf-8"),
    "utf16le": b"".join(u.to_bytes(2, "little") for u in units),
    "utf16be": b"".join(u.to_bytes(2, "big") for u in units),
    "utf32le": "".join(chr(u) for u in []).encode() or b"",
}
# utf32 of actual codepoints
cps = [ord(c) for c in row]
needles["utf32le"] = b"".join(c.to_bytes(4, "little") for c in cps)
needles["utf32be"] = b"".join(c.to_bytes(4, "big") for c in cps)
# also single codepoints
for cp in (0x2764, 0x1F92A, 0x1F389, 0x1F44B, 0x1F600, 0x1F446):
    needles[f"cp{cp:x}_le"] = cp.to_bytes(4, "little")
    needles[f"cp{cp:x}_be"] = cp.to_bytes(4, "big")

roots = [r"C:\Users\oli\Desktop\hcaptcha_research", r"C:\Users\oli\Desktop\hsw_rust\assets"]
files = []
for r in roots:
    for pat in ("**/*.js", "**/*.wasm", "**/*.json", "**/*.bin"):
        files += glob.glob(os.path.join(r, pat), recursive=True)

for p in sorted(set(files)):
    try:
        b = open(p, "rb").read()
    except Exception:
        continue
    hits = [(k, b.find(v)) for k, v in needles.items() if v and b.find(v) >= 0]
    if hits:
        print("==", p, len(b), hits)
