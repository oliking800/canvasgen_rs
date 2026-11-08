import glob, os, re, unicodedata, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

paths = []
for pat in ("hcaptcha_research/assets/hsw_*.js", "hcaptcha_research/assets/api.js",
            "hcaptcha_research/assets/hcaptcha.html", "canvasgen_rs/src/**/*.rs",
            "canvasgen_rs/samples/manifest.json"):
    paths += glob.glob(os.path.join(r"C:\Users\oli\Desktop", pat), recursive=True)

emoji_re = re.compile("[\U0001F000-\U0001FAFF\u2600-\u27BF\u2B00-\u2BFF\uFE0F\u200D\u2764]")

for p in sorted(set(paths)):
    try:
        s = open(p, encoding="utf-8", errors="replace").read()
    except Exception:
        continue
    hits = emoji_re.findall(s)
    if not hits:
        continue
    # group into runs
    runs = []
    for m in emoji_re.finditer(s):
        if runs and m.start() - runs[-1][1] <= 6:
            runs[-1] = (runs[-1][0], m.end())
        else:
            runs.append((m.start(), m.end()))
    print("==", os.path.basename(p), "chars", len(hits))
    seen = set()
    for a, b in runs:
        frag = s[a:b]
        if frag in seen:
            continue
        seen.add(frag)
        cps = " ".join(f"U+{ord(c):04X}" for c in frag)
        print(f"   {frag!r}  {cps}")
        if len(seen) > 12:
            break
