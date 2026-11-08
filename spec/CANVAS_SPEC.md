# hCaptcha canvas probe — complete captured settings

Ground truth extracted from the hCaptcha `hsw` build **82745a9a0337** and its
recorded canvas ProofSpec, then replayed against a mocked 2D context to capture
every property set, gradient stop and draw call.

Sources:

* `hcaptcha_research/proof_emulation/chrome_proof_env.json` — the browser
  capture: `cap.ctxs`, `cap.dataURLs`, `cap.ops` (137 ops with per-op state).
* `hcaptcha_research/proof_emulation/env827.json` — same program in installer
  form (the env the `n` producer consumes).
* `canvas_calls.json` (this repo, produced by `tools/capture_canvas_calls.js`)
  — the live replay trace: every `set` and every call, including concrete
  radial-gradient coordinates/stops and shadow values.
* `samples/proof/dataurl_0..4.png` — the five real PNGs decoded from the
  captured data URLs.

The live trace is **byte-identical across runs** (fonts, gradients, shadows,
coordinates and text never change), so these are fixed settings, not random:
`tools/check_canvas_determinism.py` runs the capture twice and compares.

## Canvas inventory

| # | ctx | size | URL | captured PNG | program |
|---|-----|------|-----|--------------|---------|
| 0 | html, 2d | 20×20 | 0 | 1,190 B data URL / 874 B PNG | emoji face |
| 1 | html, webgl | 300×150 | 1 | 5,982 B / 4,469 B | RGB triangle |
| 2 | html, 2d | 20×20 | 2 | 1,946 B / 1,441 B | abstract thumbnail |
| 3 | html, 2d | 300×150 | 3 | 4,630 B / 3,455 B | emoji row |
| 4 | html, 2d | 96×48 | 4 | 9,294 B / 6,953 B | abstract image |
| 5 | html, 2d | 20×20 | 5 | 1,190 B / 874 B | emoji face (repeat) |

`cap.ctxs` (exact, also `canvasgen::CANVAS_CTXS_JSON`):

```json
[{"label":"offscreen","type":"webgl2","ok":true,"w":1,"h":1},
 {"label":"html","type":"2d","ok":true,"w":300,"h":150},
 {"label":"html","type":"webgl","ok":true,"w":300,"h":150},
 {"label":"html","type":"2d","ok":true,"w":300,"h":150},
 {"label":"html","type":"2d","ok":true,"w":300,"h":150},
 {"label":"html","type":"2d","ok":true,"w":300,"h":150}]
```

The installer returns `canvasDataURLs[__canvasIdx++ % 5]`, so the sixth call
repeats the 20×20 face. The complete per-op program is in
`canvas_program.json` (context, size, URL index, args, resolved state).

## Probe 0 / 5 — 20×20 emoji face

```
font   = "15px system-ui, sans-serif"
fill   = #000000
fillText("😀", 0, 15)
fillStyle = #000000 ; fillRect(0, 0, 2, 2)
fillStyle = #ffffff ; fillRect(2, 2, 1, 1)
beginPath(); arc(0, 0, 2, 0, 1, anticlockwise=true); closePath(); fill()
getImageData(0, 0, 2, 2)
```

Captured ink: x 1..18, y 1..18 (253 non-transparent px); the emoji itself is
15×15 px at x 3..17, y 2..16.

## Probe 1 — 300×150 WebGL triangle

Non-black bounding box x 15..269, y 37..127. Apex ≈ (149.5, 37); base corners
(15, 127) and (269, 127). Fragment colour is
`(2x/300, 2(150−y)/150, 1)` (verified: (40,120) → 69,100,255;
(150,100) → 255,168,255). `canvasgen::gradient_triangle` reproduces this bbox
exactly at seed 42.

## Probe 2 — 20×20 abstract thumbnail

83 ops: 18 `beginPath`, 7 `arc`, 4 `ellipse`, 7 `moveTo`, 3
`quadraticCurveTo`, 4 `bezierCurveTo`, 20 `fill`, 18 `stroke`, 2 `strokeText`.
Drawn on a **transparent** canvas (no background `fillRect`). Each path is
filled *and* stroked with a radial gradient (`createRadialGradient` with two
radius-0/1 circles and two palette stops). Text:

```
font = 6.68896321070234px 'Segoe Fluent Icons',...,sans-serif
strokeText("xyz😃👾", 2, 10, 6)   // 4th arg = maxWidth
strokeText("xyz😃👾", 0, 12, 16)
```

Captured PNG: 20×20, alpha range 45..255 (49% opaque), dominant colours from
the gradient palette (e.g. #E6351C, #B24E50).

## Probe 3 — 300×150 emoji row

```
font       = "10px sans-serif"        // canvas default, NOT the 6.69px replay font
fillStyle  = "#000000"
beginPath()
fillText("❤️🤪🎉👋", 50, 70)
stroke()                             // empty path, no-op
```

Captured ink: x 51..102, y 60..71. Verified independently in real headless
Chrome: at `6.68896321070234px` the same text occupies only x 50..85, y 64..71,
so the captured row really is the 10 px default. `canvasgen::emoji_text` uses
`DEFAULT_EMOJI_ROW = "❤️🤪🎉👋"`, font size 10, x 50, baseline 70.

> Note: the producer's replay trace also sets a radial gradient
> (`createRadialGradient(11,7,0,7,9,1)`, `#CCFF1A → #E64D66`) and
> `shadowBlur 24.566748063684784`, `shadowColor #FFB399` before this call.
> Those are replay-only state, not present in the captured browser render
> (no shadow halo exists around the PNG's emoji); colour emoji ignore the
> fill. `canvasgen` deliberately follows the captured render.

## Probe 4 — 96×48 abstract image

47 ops: 3 `fillRect`, 8 `beginPath`, 6 `moveTo`, 8 `lineTo`, 4 `closePath`,
6 `fill`, 2 `stroke`, 2 `arc`, 2 `bezierCurveTo`, 4 `strokeText`, 2
`fillText`. Background `fillRect(0,0,96,48)` = **#fa6939**.

Palette (measured from `fillStyle`):

```
rgba(207,2,220,0.38)   rgba(33,85,92,0.37)    rgba(38,136,74,0.74)
rgba(131,14,80,0.83)   rgba(73,120,88,0.77)   rgba(100,141,191,0.75)
rgba(89,248,7,0.63)
```

Text (all with `strokeText` + `fillText` pairs where shown):

```
12.75px monospace   strokeText("╀Ň仈А", 35.1875, 15.875)
18.06px cursive     fillText  ("Ù亪Ę┿乹ч◚◅", 61, 41.0625)
18.06px cursive     strokeText("Ù亪Ę┿乹ч◚◅", 61, 41.0625)
 8.12px monospace   fillText  ("む🌶Θζ", 58.875, 40.875)
 8.12px monospace   strokeText("む🌶Θζ", 58.875, 40.875)
14.25px cursive     strokeText("Ùв┏🖘🎞Ω╊1", 17.625, 9.0625)
```

Line widths: 3, 1.125, 1.1875, 0.5625, 2.8125, 2.

## What `canvasgen` now does

* `emoji_text` — exact row text, 10 px default font, x 50 / baseline 70.
* `small_probe` — 😀 at (0,15) with `15px system-ui`, then the two rects and
  the corner arc; no longer centres a 😃.
* `abstract_image` — #fa6939 background, the seven translucent palette
  colours, polygon/arc/ellipse/bezier/rect mix, four text sizes at
  monospace/cursive families.
* `SmallBlob` — renders the abstract program directly at 20×20 on a
  transparent canvas (no 96×48 downscale).
* `gradient_triangle` — unchanged; verified against the capture's bbox.

Known approximations (a generator, not a replay): exact random shape
coordinates/vertex counts differ per seed; radial-gradient geometry per shape
is approximated with the captured two-stop palette; `strokeText` weight is
emulated by a 0.5 px offset copy. The five data URLs are still exactly the
real PNGs in `samples/proof/` when `canvas.mode = "capture"`.

## Per-class canvas bases

`epic_hcaptcha_rs/assets/canvas_bases.json` stores real five-URL sets for the
four device classes:

| class | variants | source |
|---|---|---|
| `chrome` | 1 | reference Chrome / NVIDIA capture (pixel-identical to the device-lab replay) |
| `firefox` | 2 | Firefox 158 on NVIDIA (DPR 1.25) and AMD (DPR 1) |
| `iphone` | 5 | iPhone 15 and four other iOS models, Apple GPU |
| `android` | 1 | Chrome / Adreno 740, Android |

`fingerprint.json` → `canvas.mode = "class"` selects the class from
`canvas.class` (`auto` maps iPhone/iPad → iphone, Android → android, Firefox →
firefox, otherwise chrome) and picks a variant from `canvas_seed`, so the same
profile is stable and different profiles land on different devices of the same
class.

Why this exists: `canvas.mode = "generate"` renders with the host's fonts. On
Windows that is Segoe UI Emoji + Segoe/Arial text, which is visibly wrong for an
iPhone or Android profile. The class bases carry the real Apple Color Emoji /
Noto Color Emoji artwork and the correct engine rasterization. `generate` stays
available for per-device micro-variation; `class` is the high-fidelity choice.

`canvas.vary` (default true) perturbs the chosen base inside the measured
same-class envelopes before returning it: ±1-LSB gradient contours on the WebGL
probe, edge-only text deltas, and ±1-LSB compositor differences on the dense
gradient probes. Same seed → identical output; different seeds → different
hashes. See `spec/DEVICE_VARIANCE.md` for the measured bounds.

