"""Tiny LAN server for collecting real-device canvas probes.

Run:  python tools/device_lab_server.py [--port 8788]
Then open http://<this-pc-lan-ip>:8788 on the phone (same Wi-Fi).
"""
import argparse, hashlib, json, os, re, socket, sys, time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PAGE = os.path.join(ROOT, "tools", "device_lab", "index.html")
OUTDIR = os.path.join(ROOT, "captures_devices")
os.makedirs(OUTDIR, exist_ok=True)

MAX_BODY = 4 * 1024 * 1024


def lan_ips():
    ips = []
    try:
        for info in socket.getaddrinfo(socket.gethostname(), None, socket.AF_INET):
            ip = info[4][0]
            if not ip.startswith("127.") and ip not in ips:
                ips.append(ip)
    except OSError:
        pass
    # UDP trick finds the primary outbound interface without sending packets.
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.connect(("192.168.1.1", 80))
        ip = s.getsockname()[0]
        s.close()
        if ip and not ip.startswith("127.") and ip not in ips:
            ips.insert(0, ip)
    except OSError:
        pass
    return ips


def safe_name(s, limit=80):
    s = re.sub(r"[^A-Za-z0-9._-]+", "_", s or "")
    return (s[:limit] or "unknown")


class Handler(BaseHTTPRequestHandler):
    server_version = "canvas-lab/1.0"

    def log_message(self, fmt, *args):
        return  # keep the console clean; submissions are logged explicitly

    def _send(self, code, body, ctype="application/json"):
        data = body.encode("utf-8") if isinstance(body, str) else body
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        try:
            self.wfile.write(data)
        except (BrokenPipeError, ConnectionResetError):
            pass

    def do_GET(self):
        path = self.path.split("?", 1)[0]
        if path in ("/", "/index.html"):
            try:
                body = open(PAGE, "rb").read()
            except FileNotFoundError:
                self._send(500, "page missing; run tools/gen_device_lab.py")
                return
            self._send(200, body, "text/html; charset=utf-8")
        elif path == "/health":
            self._send(200, json.dumps({"ok": True}))
        else:
            self._send(404, "not found")

    def do_POST(self):
        if self.path.split("?", 1)[0] != "/submit":
            self._send(404, "not found")
            return
        try:
            n = int(self.headers.get("Content-Length") or 0)
        except ValueError:
            n = 0
        if n <= 0 or n > MAX_BODY:
            self._send(413, json.dumps({"ok": False, "error": "bad body size"}))
            return
        raw = self.rfile.read(n)
        try:
            payload = json.loads(raw.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as e:
            self._send(400, json.dumps({"ok": False, "error": f"json: {e}"}))
            return

        ts = time.time()
        stamp = time.strftime("%Y%m%d_%H%M%S", time.localtime(ts))
        remote = self.client_address[0]
        ua = str(payload.get("ua") or "")
        # rough device label from the UA for the filename
        label = "device"
        if "iPhone" in ua:
            label = "iphone"
        elif "iPad" in ua:
            label = "ipad"
        elif "Android" in ua:
            m = re.search(r"Android[^;]*;\s*([^)]+)\)", ua)
            label = safe_name(m.group(1) if m else "android")
        filename = f"{stamp}_{safe_name(label)}_{safe_name(remote)}.json"
        doc = {
            "received_at": ts,
            "received_iso": time.strftime("%Y-%m-%dT%H:%M:%S", time.localtime(ts)),
            "remote": remote,
            "headers": {k: v for k, v in self.headers.items()},
            "payload": payload,
            "canvas_sha256": [
                hashlib.sha256((u or "").encode("utf-8")).hexdigest() for u in
                (payload.get("canvases") or [])
            ],
        }
        path = os.path.join(OUTDIR, filename)
        with open(path, "w", encoding="utf-8") as f:
            json.dump(doc, f, ensure_ascii=False, indent=1)
        # append a compact index line
        with open(os.path.join(OUTDIR, "index.jsonl"), "a", encoding="utf-8") as f:
            f.write(json.dumps({
                "file": filename,
                "received_iso": doc["received_iso"],
                "remote": remote,
                "ua": ua[:160],
                "gpu": (payload.get("gpu") or {}).get("renderer"),
                "dpr": payload.get("dpr"),
                "repeatIdentical": payload.get("repeatIdentical"),
                "canvas_sha256": doc["canvas_sha256"],
            }, ensure_ascii=False) + "\n")
        print(f"[{doc['received_iso']}] {filename}  gpu={((payload.get('gpu') or {}).get('renderer'))!r}  "
              f"repeat={payload.get('repeatIdentical')}")
        self._send(200, json.dumps({"ok": True, "file": filename,
                                    "repeatIdentical": payload.get("repeatIdentical")}))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=8788)
    ap.add_argument("--bind", default="0.0.0.0")
    args = ap.parse_args()
    try:
        page_ok = os.path.exists(PAGE)
    except OSError:
        page_ok = False
    if not page_ok:
        print("warning: tools/device_lab/index.html missing; run tools/gen_device_lab.py")
    srv = ThreadingHTTPServer((args.bind, args.port), Handler)
    print(f"serving {PAGE}")
    print(f"saving captures to {OUTDIR}")
    print("open on the phone (same Wi-Fi):")
    for ip in lan_ips():
        print(f"  http://{ip}:{args.port}")
    print("Ctrl+C to stop")
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        print("\nstopped")


if __name__ == "__main__":
    main()
