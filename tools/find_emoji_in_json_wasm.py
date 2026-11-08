import json, glob, os, re, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

files = []
for pat in (r"C:\Users\oli\Desktop\hcaptcha_research\**\*.json",
            r"C:\Users\oli\Desktop\hcaptcha_research\**\*.wasm",
            r"C:\Users\oli\Desktop\hcaptcha_research\**\*.bin"):
    files += glob.glob(pat, recursive=True)

rx = re.compile("[\U0001F000-\U0001FAFF\u2600-\u27BF\u2B00-\u2BFF]")
for p in sorted(set(files)):
    try:
        if p.endswith(".json"):
            s = open(p, encoding="utf-8", errors="replace").read()
        else:
            b = open(p, "rb").read()
            s = b.decode("utf-8", "replace")
    except Exception:
        continue
    hits = rx.findall(s)
    if hits:
        print("==", os.path.basename(p), os.path.getsize(p), "hits", len(hits))
        i = rx.search(s).start()
        print("   ctx", s[max(0, i-160):i+240].replace("\n", " ")[:400])
