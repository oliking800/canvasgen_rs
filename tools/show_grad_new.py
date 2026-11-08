import json, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
t = json.load(open(r"C:\Users\oli\Desktop\canvasgen_rs\canvas_calls.json", encoding="utf-8"))
g = [e for e in t if e["kind"] == "gradNew"]
print("gradNew", len(g))
for e in g[:3]:
    print("id", e["name"], "coords", json.dumps(e["args"][0], ensure_ascii=False)[:200])
    print("stack", (e["args"][1] or "")[:700])
    print()
