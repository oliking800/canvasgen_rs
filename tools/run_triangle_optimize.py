import base64, json, os, re, shutil, subprocess, sys, tempfile
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

ROOT = r"C:\Users\oli\Desktop\canvasgen_rs"
ref_url = "data:image/png;base64," + base64.b64encode(
    open(ROOT + r"\samples\proof\dataurl_1.png", "rb").read()).decode()

html = open(ROOT + r"\tools\triangle_optimize.html", encoding="utf-8").read()
html = html.replace('"__REF__"', json.dumps(ref_url))

tmpdir = tempfile.mkdtemp(prefix="chrome-opt-")
tmp = os.path.join(tmpdir, "t.html")
open(tmp, "w", encoding="utf-8").write(html)
chrome = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
r = subprocess.run([chrome, "--headless=new", "--no-sandbox", "--hide-scrollbars",
                    "--virtual-time-budget=30000", "--dump-dom",
                    "file:///" + tmp.replace("\\", "/")],
                   capture_output=True, text=True, timeout=180000)
m = re.search(r'data-out="([^"]+)"', r.stdout or "")
if not m:
    print("no output", (r.stdout or "")[:500], (r.stderr or "")[:300]); sys.exit(1)
raw = m.group(1).replace("&quot;", '"').replace("&amp;", "&")
res = json.loads(raw)
for x in res:
    print(f"apex ({x['ax']},{x['ay']}) left {x['lx']} right {x['rx']} base {x['by']}: "
          f"diff {x['diff']} mean {x['mean']:.3f} max {x['max']}")
shutil.rmtree(tmpdir, ignore_errors=True)
