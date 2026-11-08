import glob, os, re, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

ROOT = r"C:\Users\oli\Desktop\hcaptcha_research"
files = glob.glob(os.path.join(ROOT, "assets", "*.js")) + glob.glob(os.path.join(ROOT, "assets", "*.html"))
# add the hsw bundles and the live ones
files += glob.glob(os.path.join(ROOT, "assets", "hsw_*.js"))

nums = {
    "U+2764": ["10084", "0x2764", "2764"],
    "U+FE0F": ["65039", "0xfe0f", "fe0f"],
    "U+1F92A": ["129322", "0x1f92a", "1f92a"],
    "U+1F389": ["127881", "0x1f389", "1f389"],
    "U+1F44B": ["128075", "0x1f44b", "1f44b"],
    "fromCodePoint": ["fromCodePoint", "fromCharCode"],
}

for p in sorted(set(files)):
    try:
        s = open(p, encoding="utf-8", errors="replace").read()
    except Exception:
        continue
    found = []
    for label, pats in nums.items():
        for pat in pats:
            if pat in s:
                found.append((label, pat, s.find(pat)))
                break
    if found:
        print("==", os.path.basename(p), "len", len(s))
        for label, pat, i in found:
            print(f"   {label} via {pat!r} at {i}")
            ctx = s[max(0, i-160):i+200].replace("\n", " ")
            print("      ", ctx[:360])
