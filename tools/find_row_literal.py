import re, sys, glob, os
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

ROOT = r"C:\Users\oli\Desktop\hcaptcha_research\assets"
files = sorted(glob.glob(os.path.join(ROOT, "hsw_*.js")))

# UTF-16 units of the observed row and some alternatives
row = "❤️🤪🎉👋"
units = []
for ch in row:
    o = ord(ch)
    if o > 0xFFFF:
        o -= 0x10000
        units += [0xD800 + (o >> 10), 0xDC00 + (o & 0x3FF)]
    else:
        units.append(o)
print("row units dec", units)
print("row units hex", [hex(u) for u in units])

# search text forms: decimal comma list, hex comma list, \u escapes, actual chars
forms = []
for sep in [",", ", "]:
    forms.append(sep.join(str(u) for u in units))
    forms.append(sep.join(hex(u) for u in units))
    forms.append(sep.join(hex(u)[2:] for u in units))
forms += [
    "".join("\\u%04x" % u for u in units),
    "".join("\\u%04X" % u for u in units),
    row,
    repr(row),
]

for p in files:
    s = open(p, encoding="utf-8", errors="replace").read()
    hits = [(f, s.find(f)) for f in forms if f in s]
    if hits:
        print("==", os.path.basename(p), hits)
        f, i = hits[0]
        print("   ctx:", s[max(0, i-200):i+300].replace("\n", " ")[:520])
