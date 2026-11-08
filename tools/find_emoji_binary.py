import glob, os, re

ROOT = r"C:\Users\oli\Desktop\hcaptcha_research"
files = []
files += glob.glob(os.path.join(ROOT, "**", "*.wasm"), recursive=True)
files += glob.glob(os.path.join(ROOT, "assets", "hsw_*.js"))
files += glob.glob(os.path.join(ROOT, "assets", "*.js"))

# emoji UTF-8 needles
needles = {
    "heart": bytes.fromhex("e29da4"),
    "zany": bytes.fromhex("f09fa4aa"),
    "party": bytes.fromhex("f09f8e89"),
    "wave": bytes.fromhex("f09f918b"),
    "grin": bytes.fromhex("f09f9880"),
    "index": bytes.fromhex("f09f9186"),
}

for p in sorted(set(files)):
    try:
        b = open(p, "rb").read()
    except Exception:
        continue
    hits = {k: b.find(v) for k, v in needles.items()}
    found = {k: v for k, v in hits.items() if v >= 0}
    if found:
        print("==", os.path.basename(p), os.path.getsize(p), "hits", list(found))
        for k, i in found.items():
            a, z = max(0, i - 80), min(len(b), i + 200)
            chunk = b[a:z]
            txt = re.sub(rb"[^\x20-\x7e]", b".", chunk).decode("ascii", "replace")
            print(f"   {k}@{i}: {txt}")
