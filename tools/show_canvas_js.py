import re, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

P = r"C:\Users\oli\Desktop\hcaptcha_research\assets\hsw_00aabc05.js"
s = open(P, encoding="utf-8", errors="replace").read()

for pat in ["fillText", "strokeText", "measureText", "fillRect", "createLinearGradient",
            "globalCompositeOperation", "shadowBlur", "shadowColor", "font=",
            "textBaseline", "textAlign", "canvas", "toDataURL", "createElement"]:
    idxs = [m.start() for m in re.finditer(re.escape(pat), s)]
    print(f"== {pat}: {len(idxs)} at {idxs[:8]}")
    for i in idxs[:4]:
        ctx = s[max(0, i - 260):i + 260].replace("\n", " ")
        print("   ...", ctx[:520])
    print()
