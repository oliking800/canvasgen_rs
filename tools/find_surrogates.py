import re, sys, glob, os
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

ROOT = r"C:\Users\oli\Desktop\hcaptcha_research\assets"
files = sorted(glob.glob(os.path.join(ROOT, "hsw_*.js")))

patterns = {
    "esc_upper": re.compile(r"\\uD83[0-9A-F]\\uD[Ee][0-9A-F]{2}"),
    "esc_lower": re.compile(r"\\ud83[0-9a-f]\\ud[ef][0-9a-f]{2}"),
    "u_brace": re.compile(r"\\u\{1[fF][0-9a-fA-F]{3}\}"),
    "fromCodePoint": re.compile(r"fromCodePoint"),
    "codePointAt": re.compile(r"codePointAt"),
}

for p in files[:1]:
    s = open(p, encoding="utf-8", errors="replace").read()
    print("file", os.path.basename(p), "len", len(s))
    for name, rx in patterns.items():
        ms = list(rx.finditer(s))
        print(f"  {name}: {len(ms)}")
        for m in ms[:6]:
            print("     @", m.start(), repr(s[max(0, m.start()-90):m.end()+90]))
