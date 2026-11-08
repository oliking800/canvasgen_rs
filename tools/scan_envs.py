import json, glob, os, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

files = glob.glob(r"C:\Users\oli\Desktop\hcaptcha_research\**\env*.json", recursive=True)
files += glob.glob(r"C:\Users\oli\Desktop\hcaptcha_research\**\*proof*.json", recursive=True)
seen = set()
for p in sorted(set(files)):
    try:
        d = json.load(open(p, encoding="utf-8"))
    except Exception:
        continue
    cap = d.get("cap") if isinstance(d, dict) else None
    if not isinstance(cap, dict) or "ops" not in cap:
        continue
    ops = cap["ops"]
    if not ops:
        continue
    texts = [(i, o[0], o[1]) for i, o in enumerate(ops)
             if o[0] in ("fillText", "strokeText")]
    fonts = sorted({o[2].get("font") for o in ops if len(o) > 2 and isinstance(o[2], dict) and "font" in o[2]})
    sizes = [(u.get("len"), u.get("head", "")[:40]) for u in cap.get("dataURLs", []) if isinstance(u, dict)]
    key = (len(ops), tuple((t[0], str(t[2])) for t in texts))
    print("==", os.path.relpath(p, r"C:\Users\oli\Desktop\hcaptcha_research"), "ops", len(ops), "urls", sizes)
    for i, n, a in texts:
        print(f"   {i:3d} {n:10s} {json.dumps(a, ensure_ascii=False)[:90]}")
    print("   fonts:")
    for f in fonts:
        print("     ", (f or "")[:100])
