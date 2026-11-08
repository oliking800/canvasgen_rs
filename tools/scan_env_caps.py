import json, glob, os, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

files = glob.glob(r"C:\Users\oli\Desktop\hcaptcha_research\**\env*.json", recursive=True)
for p in sorted(set(files)):
    try:
        d = json.load(open(p, encoding="utf-8"))
    except Exception:
        continue
    if not isinstance(d, dict):
        continue
    cap = d.get("cap")
    urls = d.get("canvasDataURLs") or (cap.get("canvasDataURLs") if isinstance(cap, dict) else None)
    ctxs = d.get("canvasCtxs") or (cap.get("ctxs") if isinstance(cap, dict) else None)
    ops = cap.get("ops") if isinstance(cap, dict) else None
    if urls or ctxs or ops:
        print("==", os.path.relpath(p, r"C:\Users\oli\Desktop\hcaptcha_research"))
        print("   urls", (len(urls) if urls else None), [len(u) if isinstance(u, str) else (u.get("len") if isinstance(u, dict) else "?") for u in (urls or [])][:8])
        print("   ctxs", (len(ctxs) if ctxs else None), "ops", (len(ops) if ops else None))
        if ops:
            print("   canvas texts", [o[1][0] for o in ops if o[0] in ("fillText","strokeText")][:12])
