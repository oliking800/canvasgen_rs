"""Build epic_hcaptcha_rs/assets/canvas_bases.json from the real-device
captures plus the original Chrome reference capture.
"""
import base64, glob, json, os, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

ROOT = r"C:\Users\oli\Desktop"
CANVAS = os.path.join(ROOT, "canvasgen_rs")
DEV = os.path.join(CANVAS, "captures_devices")
OUT = os.path.join(ROOT, "epic_hcaptcha_rs", "assets", "canvas_bases.json")
os.makedirs(os.path.dirname(OUT), exist_ok=True)

def read_capture(name):
    d = json.load(open(os.path.join(DEV, name), encoding="utf-8"))
    p = d["payload"]
    return {
        "label": name,
        "ua": p.get("ua"),
        "gpu": (p.get("gpu") or {}).get("renderer"),
        "dpr": p.get("dpr"),
        "screen": p.get("screen"),
        "urls": p.get("canvases"),
    }

def png_url(path):
    b = open(path, "rb").read()
    return "data:image/png;base64," + base64.b64encode(b).decode()

chrome_urls = [png_url(os.path.join(CANVAS, "samples", "proof", f"dataurl_{i}.png")) for i in range(5)]

bases = {
    "schema": "hcaptcha-canvas-bases/1",
    "note": "Five real canvasDataURLs per device class. Captured with the "
            "device-lab page, which reproduces the reference Chrome capture "
            "pixel-for-pixel.",
    "classes": {
        "chrome": {
            "variants": [{
                "label": "chrome-win-nvidia",
                "ua": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
                      "(KHTML, like Gecko) Chrome/157.0.0.0 Safari/537.36",
                "gpu": "ANGLE (NVIDIA GeForce RTX 4060, D3D11)",
                "dpr": 1,
                "urls": chrome_urls,
            }]
        },
        "firefox": {
            "variants": [
                read_capture("20261108_112545_device_127.0.0.1.json"),
                read_capture("20261108_113727_device_127.0.0.1.json"),
            ]
        },
        "iphone": {
            "variants": [
                read_capture("20261108_112211_iphone_192.168.18.19.json"),
                read_capture("20261108_113327_iphone_127.0.0.1.json"),
                read_capture("20261108_114127_iphone_127.0.0.1.json"),
                read_capture("20261108_114128_iphone_127.0.0.1.json"),
                read_capture("20261108_114151_iphone_127.0.0.1.json"),
            ]
        },
        "android": {
            "variants": [
                read_capture("20261108_114203_K_127.0.0.1.json"),
            ]
        },
    },
}

# sanity: every variant has 5 URLs
for cls, c in bases["classes"].items():
    for v in c["variants"]:
        assert len(v["urls"]) == 5, (cls, v.get("label"))
        for u in v["urls"]:
            assert u.startswith("data:image/png;base64,"), (cls, u[:20])

json.dump(bases, open(OUT, "w", encoding="utf-8"), ensure_ascii=False)
print("wrote", OUT, os.path.getsize(OUT), "bytes")
for cls, c in bases["classes"].items():
    print(f"  {cls}: {len(c['variants'])} variants")
    for v in c["variants"]:
        print(f"    {v['label']}: dpr={v.get('dpr')} gpu={v.get('gpu')}")
