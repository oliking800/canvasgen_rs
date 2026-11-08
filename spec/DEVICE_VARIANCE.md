# Real-device canvas variance (calibration)

Captured with `tools/device_lab_server.py` + `tools/gen_device_lab/index.html`.
The page replays the exact hCaptcha canvas program; on the capture machine it
reproduces the original five PNGs pixel-for-pixel (874 / 4469 / 1441 / 3455 /
6953 bytes), so every difference below is real rasterizer/GPU variance, not a
program difference.

Devices:

| label | UA | GPU | DPR |
|---|---|---|---|
| chrome | HeadlessChrome 157, Windows | ANGLE / NVIDIA RTX 4060, D3D11 | 1 |
| firefox | Firefox 158, Windows | ANGLE / NVIDIA GTX 980, D3D11 | 1.25 |
| iphone | Safari 18.7, iPhone 15 (iOS 18.7) | Apple GPU | 3 |

Same-device repeat (two iPhone visits 17 s apart): **0 differing pixels on all
five probes** and `repeatIdentical = [true × 5]`.

## Measured pairwise differences

Values are max-channel |Δ| after compositing on white. `diff` is the fraction
of pixels with any difference.

| probe | pair | diff | mean | p99 | max |
|---|---|---|---|---|---|
| 0 face 20×20 | chrome vs iphone | 0.6775 | 57.7 | 238 | 255 |
| 0 face 20×20 | chrome vs firefox | 0.3850 | 4.4 | 34 | 51 |
| 1 triangle | chrome vs iphone | 0.0389 | 0.039 | 1 | **1** |
| 1 triangle | chrome vs firefox | **0.0000** | 0 | 0 | 0 |
| 2 abstract 20×20 | chrome vs iphone | 1.0000 | 17.9 | 104 | 150 |
| 2 abstract 20×20 | chrome vs firefox | 1.0000 | 15.9 | 65 | 84 |
| 3 emoji row | chrome vs iphone | 0.0123 | 1.36 | 35 | 254 |
| 3 emoji row | chrome vs firefox | 0.0084 | 0.28 | 0 | 141 |
| 4 abstract 96×48 | chrome vs iphone | 0.4646 | 16.0 | 131 | 213 |
| 4 abstract 96×48 | chrome vs firefox | 0.5122 | 8.2 | 85 | 122 |

## What each axis is

### 1. Same device — 0

Canvas rendering is deterministic. A device seed must produce bit-identical
output across visits and across the five probes.

### 2. GPU — WebGL only, ±1 LSB

* Chrome vs Firefox on the same NVIDIA GPU: **0 pixels differ**. The browser
  engine is irrelevant here.
* Apple GPU vs NVIDIA: 1,749 / 45,000 pixels (3.89 %), and **every one of them
  differs by exactly 1 LSB**; none by 2. The differing pixels are not random:
  they form long horizontal/vertical contours where the interpolated channel
  crosses an 8-bit rounding boundary (see `tools/analyze_webgl_dither.py`).
* Fitting `round(ideal + bias)` gives an effective green-channel bias of
  **−0.20 LSB (NVIDIA)** vs **0.00 LSB (Apple)**; red/blue are within 0.05 LSB.

Model: per-device `rgb_bias` in fractional LSB (`|bias| ≲ 0.25`), applied to the
interpolated colour **before** quantization. Same GPU class → same bias → 0
difference; different class → 1-LSB contour flips on a few percent of pixels.
No random noise.

### 3. Text rasterizer — edge pixels, small deltas

* 300×150 emoji row: 0.8–1.2 % of the canvas differs (≈ 30–45 % of the ink),
  mean |Δ| 0.28–1.36, most deltas ≤ 16, a handful up to 255 at hinted stems.
* Chrome vs Firefox (same fonts, different text engine) already shows this:
  subpixel positioning / hinting / gamma, not silicon.
* 20×20 face, same fonts (Chrome vs Firefox): 38.5 % of pixels, mean 4.4,
  max 51 — the same edge effect, amplified because the glyph fills the canvas.

Model: per-device `text_dx/text_dy` (±0.05 px), hinting mode and a small
alpha-gamma (`0.98–1.02`). Differences must stay on glyph edges.

### 4. Emoji artwork / OS font class — coarse

* iPhone vs desktop: 67.8 % of the 20×20 face differs, mean 57.7, max 255 —
  Apple Color Emoji vs Segoe UI Emoji artwork.
* The 300×150 row is much less affected (1.2 %, mean 1.36) because the colour
  bitmap glyphs are similar; the difference is mostly edge AA.

Model: a discrete `emoji_class ∈ {Segoe, Apple, Noto, Twemoji}` chosen with the
platform. Do not try to reach this with continuous noise.

### 5. 2D compositor engine — dense gradients

* Abstract probes: **100 %** of pixels differ even between Chrome and Firefox
  on the same PC and the same fonts (probe 2 mean 15.9, probe 4 mean 8.2).
* This is Skia vs Gecko gradient interpolation / premultiplied-alpha rounding,
  accumulated over many translucent layers.
* It is an *engine* axis, not a per-device axis: two Chrome installs on the
  same GPU produce identical output.

Model: `engine_class ∈ {Skia, Gecko, WebKit}` with per-pixel fixed rounding
offsets (±1–2 LSB) applied during compositing. Within one engine class, the
result is bit-identical.

## Simulator recipe

```
device = (platform_class, gpu_class, engine_class, seed)
```

1. Coarse classes from real-world distributions:
   platform → emoji/font class + engine class; GPU vendor → gpu class.
2. Continuous micro parameters from `seed`:
   * `rgb_bias[3] ∈ ±0.25 LSB` (GPU interpolation rounding),
   * `text_dx/dy ∈ ±0.05 px`, `hint: enum`, `gamma ∈ 0.98..1.02`,
   * compositor rounding mode (only varies across engine classes).
3. Apply each parameter inside its subsystem, before quantization. Never add
   global pixel noise.
4. Keep the seed fixed per device; all five probes must use it.

## Acceptance envelope (must hold for generated devices)

| test | bound |
|---|---|
| same seed | 0 differing pixels |
| triangle, same GPU class | 0 |
| triangle, different GPU class | all Δ = 1 LSB; differing fraction 0–8 % |
| text row, different text engine | diff ≤ 2 %; p99 ≤ 64; mean ≤ 2 |
| 20×20 face, same emoji class | diff ≤ 45 %; mean ≤ 6; max ≤ 64 |
| 20×20 face, different emoji class | diff 40–80 %; mean 4–60; max ≤ 255 |
| dense-gradient probes, same engine | 0 |
| dense-gradient probes, different engine | diff ≈ 100 %; mean 5–20 |
| per-device repeat | 0 |

These bounds come directly from the table above; a generated device that falls
outside them is "too far" and should be rejected/resampled.

## Second round: 9 devices, 12 captures

Additional devices: 5 distinct iPhones (390×844, 393×852, 375×812, 375×667@2,
402×874), Android Chrome / Adreno 360×780, and Firefox on an AMD GPU.

| group | probe 1 triangle | probe 0 face | probe 2 abstract |
|---|---|---|---|
| iPhone ↔ iPhone (10 pairs) | frac 0–3.57 %, max **1** | frac 0–65 %, mean 0–10.8 | frac 8–90 %, mean 0.1–12.2 |
| iPhone ↔ Android | frac 1.5–4.4 %, max **1–2** | frac 70–73 %, mean 60–64 | 100 %, mean 21.5–22.8 |
| Firefox NVIDIA ↔ Firefox AMD | frac 1.71 %, max **1** | frac 63 %, mean 71.8 | 100 %, mean 11.6 |
| Chrome ↔ Firefox (same PC) | **0** | 38.5 %, mean 4.4 | 100 %, mean 15.9 |

Two conclusions that change the model:

1. **Per-unit silicon defects are below 8-bit.** Several iPhone pairs are
   *bit-identical* on all probes; others differ only by 1-LSB WebGL contours.
   Different phones of the same generation share a signature.
2. **Variation is class-level.** The big probe-0/2 differences track
   OS/font/emoji set, browser engine, DPI and GPU family — not an individual
   unit's transistor variation. A continuous per-device noise knob would be
   wrong: it would make every device unique, which the data contradicts.

DPR also matters for 2D text: the two Firefox captures (DPR 1 vs 1.25) differ
on 63 % of the 20×20 face with mean 71.8, while the same GPU in another browser
differs with mean 4.4.

## Implemented in `canvasgen`

`GpuClass` (`profile.rs`) enumerates `Nvidia / Amd / Intel / Apple / Adreno /
Mali / Unknown` with fixed sub-LSB biases calibrated above
(`SiliconProfile::for_gpu`). `SiliconProfile::from_seed` picks a class
deterministically (weighted by market share), so devices of the same class
produce identical triangles and different classes produce 1-LSB contours.
`gradient_triangle` applies the bias before quantization.

Test `silicon_bias_matches_measured_gpu_signature`: Apple vs NVIDIA gives
**4.49 % of pixels differing, every delta exactly 1 LSB**; measured was 3.89 % /
max 1. Same class (NVIDIA vs NVIDIA) is byte-identical.

## Text layer (implemented)

`TextEngine` (`Skia / Gecko / WebKit`) and `TextStyle` (`dx`, `dy`, `gamma`,
`raster_delta`, `salt`) model the 2D text axis:

* origin offset `dx/dy` (subpixel glyph placement),
* alpha gamma (edge intensity),
* `raster_delta`: an edge-weighted, deterministic per-pixel delta standing in
  for another engine's colour-glyph rasterizer (flat areas get 1.5 %, edges
  100 % of the amplitude),
* `TextStyle::from_seed` selects an engine class (class-level, like the GPU:
  two installs of the same engine are identical). Use the public fields to
  model a non-default DPI/font-smoothing setup explicitly.

Calibrated against the Chrome↔Firefox pair (same fonts, same GPU):

| probe | simulated | measured |
|---|---|---|
| 300×150 emoji row | 0.84 % pixels, mean 0.271, max 244 | 0.84 %, mean 0.28, max 141 |
| 20×20 face | 40.5 % pixels, mean 4.65, max 43 | 38.5 %, mean 4.4, max 51 |

`emoji_text`/`small_probe` remain the neutral (Skia) render; `fingerprint_set`
uses `TextStyle::from_seed`, so generated profiles get the engine class. Test:
`text_style_matches_measured_engine_signature`.

Caveat: the large iPhone-vs-desktop face difference (mean 57.7) is **emoji
artwork** (Apple Color Emoji vs Segoe UI Emoji), not rasterization. Modelling it
requires the corresponding font; on a Windows host the artwork stays Segoe
unless `profile.font` points at an Apple/Noto emoji file. Similarly, the two
Firefox machines above differ because of DPI/font-smoothing configuration, not a
smooth function of `dpr`; set `TextStyle` fields explicitly for those profiles.

## Class bases + `vary` (implemented)

`epic_hcaptcha_rs/assets/canvas_bases.json` holds real five-URL sets for
`chrome` (1 variant), `firefox` (2), `iphone` (5) and `android` (1).
`canvas.mode = "class"` picks the class from the UA (or `canvas.class`) and a
variant from `canvas_seed`; `canvas.vary` (default true) then applies
`canvasgen::VaryProfile` to the chosen base.

The variation is bounded by the measured same-class envelopes:

| probe | simulated on the real bases (seed 7) | measured same-class |
|---|---|---|
| 0 face 20×20 | 25–26 % pixels, mean 1.5–1.9, max 24 | 0–65 %, mean 0–10.8, max 88 |
| 1 triangle | 1.2–4.2 %, **max 1** | 0–3.6 % (Apple↔Apple), max 1 |
| 2 abstract 20×20 | 78–83 %, mean 0.8, max 1 | 8–90 %, mean 0.1–12.2 |
| 3 text row | 0.6–0.8 %, mean 0.03–0.05, max 24 | 0–1.1 %, mean 0–0.24 |
| 4 abstract 96×48 | 100 %, mean 1.0, max 1 | 5.9–32 %, mean 1.6–6.4 (iOS↔iOS) |

Implementation:
* triangle — re-quantize the known gradient with a new sub-LSB bias, only for
  interior pixels that still match the ideal; flips follow quantization
  contours and never exceed 1 LSB;
* text — edge-weighted deterministic delta on glyph pixels only;
* abstract — ±1-LSB compositor gain/offset on non-transparent pixels.

Same seed → byte-identical (`vary_canvas_data_url` is deterministic;
`pngout` has fixed encoder settings). Different seeds → different hashes,
still inside the envelope above. `"vary": false` returns the captured base.



