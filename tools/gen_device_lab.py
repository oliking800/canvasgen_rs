"""Build the phone-facing calibration page from spec/canvas_program.json.

The page replays the exact captured 2D programs (same args, gradients, fonts,
shadows) plus an equivalent WebGL triangle, renders everything twice, and POSTs
the data URLs and device info to /submit.
"""
import json, os, sys

ROOT = r"C:\Users\oli\Desktop\canvasgen_rs"
SPEC = os.path.join(ROOT, "spec", "canvas_program.json")
OUT = os.path.join(ROOT, "tools", "device_lab")
os.makedirs(OUT, exist_ok=True)

progs = json.load(open(SPEC, encoding="utf-8"))

# Rebuild per-context command lists. One entry per data URL, in URL order.
by_url = {}
for c in progs:
    idx = c.get("data_url_index")
    if idx is None or idx not in range(5):
        continue
    if c["type"] == "webgl":
        by_url[idx] = {"kind": "webgl", "w": c["width"], "h": c["height"]}
        continue
    ops = []
    for o in c["ops"]:
        st = {}
        for k, v in (o.get("state") or {}).items():
            if k in ("font", "lineWidth", "shadowBlur", "shadowColor",
                     "textAlign", "textBaseline", "globalAlpha",
                     "fillStyle", "strokeStyle"):
                st[k] = v
        ops.append({"op": o["op"], "args": o["args"], "state": st})
    # URL 0 is the emoji-only face. The fillRect/arc tail in the replay
    # belongs to the sixth canvas (index 5), whose URL is never read.
    if c["width"] == 20 and c["height"] == 20 and any(
            o["op"] == "fillText" and o["args"] and o["args"][0] == "\U0001F600" for o in ops):
        ops = [o for o in ops if o["op"] == "fillText"]
    by_url[idx] = {"kind": "2d", "w": c["width"], "h": c["height"], "ops": ops}

ordered = [by_url[i] for i in range(5)]
payload = json.dumps(ordered, ensure_ascii=False, separators=(",", ":")).replace("</", "<\\/")

html = r"""<!doctype html>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>canvas device lab</title>
<style>
 body{font:15px system-ui,sans-serif;margin:16px;background:#111;color:#eee}
 h1{font-size:18px;margin:0 0 8px}
 #status{white-space:pre-wrap;background:#1d1d1d;padding:10px;border-radius:8px}
 canvas{display:block;margin:8px 0;background:#222;max-width:100%}
 .row{font-size:12px;color:#aaa}
</style>
<h1>canvas device lab</h1>
<div class="row">collecting device canvas probes…</div>
<div id="status">rendering</div>
<div id="preview"></div>

<script>
const PROGRAMS = __PROGRAMS__;

function makeGradient(ctx, g) {
  const c = g.coords || [0,0,0,0,0,1];
  const gr = ctx.createRadialGradient(c[0],c[1],c[2],c[3],c[4],c[5]);
  for (const [off, color] of (g.stops || [])) gr.addColorStop(off, color);
  return gr;
}

function applyOp(ctx, op) {
  const a = op.args || [];
  switch (op.op) {
    case 'fillText': ctx.fillText(a[0], a[1], a[2], ...(a.length > 3 ? [a[3]] : [])); break;
    case 'strokeText': ctx.strokeText(a[0], a[1], a[2], ...(a.length > 3 ? [a[3]] : [])); break;
    case 'fillRect': ctx.fillRect(a[0], a[1], a[2], a[3]); break;
    case 'strokeRect': ctx.strokeRect(a[0], a[1], a[2], a[3]); break;
    case 'clearRect': ctx.clearRect(a[0], a[1], a[2], a[3]); break;
    case 'beginPath': ctx.beginPath(); break;
    case 'closePath': ctx.closePath(); break;
    case 'moveTo': ctx.moveTo(a[0], a[1]); break;
    case 'lineTo': ctx.lineTo(a[0], a[1]); break;
    case 'quadraticCurveTo': ctx.quadraticCurveTo(a[0], a[1], a[2], a[3]); break;
    case 'bezierCurveTo': ctx.bezierCurveTo(a[0], a[1], a[2], a[3], a[4], a[5]); break;
    case 'arc': ctx.arc(a[0], a[1], a[2], a[3], a[4], !!a[5]); break;
    case 'ellipse': ctx.ellipse(a[0], a[1], a[2], a[3], a[4], a[5], a[6], !!a[7]); break;
    case 'rect': ctx.rect(a[0], a[1], a[2], a[3]); break;
    case 'fill': ctx.fill(); break;
    case 'stroke': ctx.stroke(); break;
    case 'save': ctx.save(); break;
    case 'restore': ctx.restore(); break;
    case 'getImageData': ctx.getImageData(a[0]|0, a[1]|0, a[2]|0, a[3]|0); break;
    default: /* toDataURL handled outside */ break;
  }
}

function render2d(prog) {
  const cv = document.createElement('canvas');
  cv.width = prog.w; cv.height = prog.h;
  const ctx = cv.getContext('2d');
  if (!ctx) return {error: 'no 2d context'};
  const last = {};
  for (const op of prog.ops) {
    const st = op.state || {};
    for (const k of ['font','lineWidth','shadowBlur','shadowColor','textAlign','textBaseline','globalAlpha']) {
      if (k in st && st[k] !== last[k]) { ctx[k] = st[k]; last[k] = st[k]; }
    }
    for (const k of ['fillStyle','strokeStyle']) {
      if (!(k in st)) continue;
      const v = st[k];
      const key = (v && typeof v === 'object') ? JSON.stringify(v) : String(v);
      if (key !== last[k]) {
        ctx[k] = (v && typeof v === 'object') ? makeGradient(ctx, v) : v;
        last[k] = key;
      }
    }
    applyOp(ctx, op);
  }
  return {canvas: cv, dataURL: cv.toDataURL('image/png')};
}

function renderGL(prog) {
  const cv = document.createElement('canvas');
  cv.width = prog.w; cv.height = prog.h;
  const gl = cv.getContext('webgl', {antialias: true, preserveDrawingBuffer: true});
  if (!gl) return {error: 'no webgl'};
  const vsrc = 'attribute vec2 p;void main(){gl_Position=vec4(p,0.0,1.0);}';
  const fsrc = 'precision highp float;uniform vec2 u;void main(){gl_FragColor=vec4(2.0*gl_FragCoord.x/u.x,2.0*gl_FragCoord.y/u.y,1.0,1.0);}';
  function sh(type, src) {
    const s = gl.createShader(type); gl.shaderSource(s, src); gl.compileShader(s);
    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s));
    return s;
  }
  const pr = gl.createProgram();
  gl.attachShader(pr, sh(gl.VERTEX_SHADER, vsrc));
  gl.attachShader(pr, sh(gl.FRAGMENT_SHADER, fsrc));
  gl.linkProgram(pr);
  if (!gl.getProgramParameter(pr, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(pr));
  gl.useProgram(pr);
  const w = prog.w, h = prog.h;
  const px = [[150,37.5],[15,127.5],[270,127.5]];
  const verts = new Float32Array(px.flatMap(([x,y]) => [2*x/w-1, 1-2*y/h]));
  const buf = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, buf);
  gl.bufferData(gl.ARRAY_BUFFER, verts, gl.STATIC_DRAW);
  const loc = gl.getAttribLocation(pr, 'p');
  gl.enableVertexAttribArray(loc);
  gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
  gl.uniform2f(gl.getUniformLocation(pr, 'u'), w, h);
  gl.viewport(0, 0, w, h);
  gl.clearColor(0,0,0,1); gl.clear(gl.COLOR_BUFFER_BIT);
  gl.drawArrays(gl.TRIANGLES, 0, 3);
  return {canvas: cv, dataURL: cv.toDataURL('image/png'), gl};
}

function gpuInfo(gl) {
  try {
    const dbg = gl.getExtension('WEBGL_debug_renderer_info');
    return {
      vendor: dbg ? gl.getParameter(dbg.UNMASKED_VENDOR_WEBGL) : gl.getParameter(gl.VENDOR),
      renderer: dbg ? gl.getParameter(dbg.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER),
      version: gl.getParameter(gl.VERSION),
      shading: gl.getParameter(gl.SHADING_LANGUAGE_VERSION),
      maxTexture: gl.getParameter(gl.MAX_TEXTURE_SIZE),
      maxViewport: Array.from(gl.getParameter(gl.MAX_VIEWPORT_DIMS) || []),
      aliasedPoint: gl.getParameter(gl.ALIASED_POINT_SIZE_RANGE) ? Array.from(gl.getParameter(gl.ALIASED_POINT_SIZE_RANGE)) : null,
    };
  } catch (e) { return {error: String(e)}; }
}

function fontProbe() {
  const cv = document.createElement('canvas');
  const ctx = cv.getContext('2d');
  const out = {};
  for (const [name, font, text] of [
    ['row10', '10px sans-serif', '\u2764\uFE0F\u{1F92A}\u{1F389}\u{1F44B}'],
    ['row15', '15px system-ui, sans-serif', '\u2764\uFE0F\u{1F92A}\u{1F389}\u{1F44B}'],
    ['mono', '12.75px monospace', 'xyz\u{1F603}\u{1F47E}'],
    ['cursive', '18.06px cursive', '\u00D9\u4EAA\u0118\u253F\u4E79\u0447\u251A\u2515'],
    ['emoji', '15px "Segoe UI Emoji", "Noto Color Emoji", sans-serif', '\u{1F600}'],
  ]) {
    ctx.font = font;
    const m = ctx.measureText(text);
    out[name] = {width: m.width, ascent: m.actualBoundingBoxAscent,
                 descent: m.actualBoundingBoxDescent};
  }
  return out;
}

function renderAll() {
  const first = [], second = [];
  let gl = null;
  for (const p of PROGRAMS) {
    if (p.kind === 'webgl') {
      const r = renderGL(p);
      if (r.canvas) gl = r.gl;
      first.push(r.dataURL || null);
      const r2 = renderGL(p);
      second.push(r2.dataURL || null);
    } else {
      const r = render2d(p);
      first.push(r.dataURL || null);
      const r2 = render2d(p);
      second.push(r2.dataURL || null);
    }
  }
  return {first, second, gl};
}

(function main() {
  const status = document.getElementById('status');
  try {
    const t0 = performance.now();
    const {first, second, gl} = renderAll();
    const ms = performance.now() - t0;
    const repeat = first.map((u, i) => u === second[i]);
    const payload = {
      ua: navigator.userAgent,
      platform: navigator.platform,
      language: navigator.language,
      languages: navigator.languages,
      dpr: window.devicePixelRatio,
      screen: {w: screen.width, h: screen.height, aw: screen.availWidth, ah: screen.availHeight, cd: screen.colorDepth, pd: screen.pixelDepth},
      window: {iw: innerWidth, ih: innerHeight, ow: outerWidth, oh: outerHeight},
      hardwareConcurrency: navigator.hardwareConcurrency,
      deviceMemory: navigator.deviceMemory,
      touch: navigator.maxTouchPoints,
      gpu: gl ? gpuInfo(gl) : null,
      fonts: fontProbe(),
      canvases: first,
      repeatIdentical: repeat,
      renderMs: ms,
      ts: Date.now(),
    };
    const preview = document.getElementById('preview');
    for (const u of first) {
      if (!u) continue;
      const img = new Image();
      img.src = u;
      img.style.width = Math.min(300, innerWidth - 40) + 'px';
      preview.appendChild(img);
    }
    fetch('/submit', {method: 'POST', headers: {'Content-Type': 'application/json'},
                      body: JSON.stringify(payload)})
      .then(r => r.json())
      .then(j => { status.textContent = 'submitted\n' + JSON.stringify(j, null, 1); })
      .catch(e => { status.textContent = 'submit failed: ' + e; });
  } catch (e) {
    status.textContent = 'error: ' + (e && e.stack || e);
  }
})();
</script>
"""

html = html.replace("__PROGRAMS__", payload)
open(os.path.join(OUT, "index.html"), "w", encoding="utf-8").write(html)
print("wrote", os.path.join(OUT, "index.html"), len(html), "bytes")
