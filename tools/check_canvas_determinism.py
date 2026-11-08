import json, subprocess, sys, os
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

def norm(t):
    return [(e["kind"], str(e["name"]), e["args"]) for e in t
            if e["kind"] != "toDataURL"]

# run capture twice
for k in (1, 2):
    subprocess.run(["node", r"C:\Users\oli\Desktop\canvasgen_rs\tools\capture_canvas_calls.js"],
                   check=True, cwd=r"C:\Users\oli\Desktop\canvasgen_rs",
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                   shell=True)
    t = json.load(open(r"C:\Users\oli\Desktop\canvasgen_rs\canvas_calls.json", encoding="utf-8"))
    json.dump(norm(t), open(rf"C:\Users\oli\Desktop\canvasgen_rs\canvas_norm_{k}.json", "w", encoding="utf-8"),
              ensure_ascii=False, indent=0)
    print("run", k, "events", len(t))

a = json.load(open(r"C:\Users\oli\Desktop\canvasgen_rs\canvas_norm_1.json", encoding="utf-8"))
b = json.load(open(r"C:\Users\oli\Desktop\canvasgen_rs\canvas_norm_2.json", encoding="utf-8"))
print("identical:", a == b)
if a != b:
    n = 0
    for i, (x, y) in enumerate(zip(a, b)):
        if x != y:
            print("first diff", i)
            print("  A", json.dumps(x, ensure_ascii=False)[:200])
            print("  B", json.dumps(y, ensure_ascii=False)[:200])
            n += 1
            if n > 5:
                break

