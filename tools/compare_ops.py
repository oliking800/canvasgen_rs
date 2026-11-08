import json, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

env = json.load(open(r"C:\Users\oli\Desktop\hcaptcha_research\proof_emulation\chrome_proof_env.json", encoding="utf-8"))
ops = env["cap"]["ops"]
t = json.load(open(r"C:\Users\oli\Desktop\canvasgen_rs\canvas_calls.json", encoding="utf-8"))
skip = {"toDataURL", "getContext", "createLinearGradient", "createRadialGradient",
        "clearRect", "getImageData"}
calls = [e for e in t if e["kind"] == "call" and e["name"] not in skip]
print("env ops", len(ops), "trace calls (filtered)", len(calls))

def norm(a):
    if isinstance(a, bool):
        return "true" if a else "false"
    if isinstance(a, (int, float)):
        return round(float(a), 10)
    return a

n = min(len(ops), len(calls))
diffs = 0
for i in range(n):
    on, oa = ops[i][0], ops[i][1]
    cn, ca = calls[i]["name"], calls[i]["args"]
    if on != cn or [norm(x) for x in oa] != [norm(x) for x in ca]:
        if diffs < 20:
            print(f"{i}: env {on}{oa}  != trace {cn}{ca}")
        diffs += 1
print("mismatches:", diffs, "of", n)
