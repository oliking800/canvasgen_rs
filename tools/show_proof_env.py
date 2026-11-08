import json, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

P = r"C:\Users\oli\Desktop\hcaptcha_research\proof_emulation\chrome_proof_env.json"
d = json.load(open(P, encoding="utf-8"))
print("top-level keys:", list(d.keys()))

def walk(o, path="", depth=0):
    if depth > 2:
        return
    if isinstance(o, dict):
        for k, v in o.items():
            t = type(v).__name__
            n = len(v) if isinstance(v, (list, dict, str)) else v
            print(f"{'  '*depth}{path}{k}: {t} {n if not isinstance(v, dict) else ''}")
            if isinstance(v, dict):
                walk(v, path + k + ".", depth + 1)

walk(d)
