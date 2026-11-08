import json, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
d = json.load(open(r"C:\Users\oli\Desktop\canvasgen_rs\spec\canvas_program.json", encoding="utf-8"))
for c in d:
    if c["ctx_index"] in (0, 5):
        print("ctx", c["ctx_index"], c["width"], c["height"], "url", c["data_url_index"])
        for o in c["ops"]:
            print("   ", o["op"], o["args"])
