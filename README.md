# canvasgen

Deterministic, plausible browser **canvas fingerprints** in pure Rust — including
**full-colour emoji** text rendered from real COLR/CPAL, CBDT/CBLC and `sbix`
fonts. No browser, no JavaScript, no bundled fonts.

`canvasgen` reproduces the five `canvasDataURLs` hCaptcha asks a browser for,
keeps the same sizes/order, and can render an emoji row that is visually almost
indistinguishable from the captured Chrome probe.

> **Status: source published on GitHub (crates.io not published).**
> The package is verified with `cargo package` and `cargo publish --dry-run`;
> no crates.io upload is performed. See
> [Packaging status](#packaging-status-github-source-not-cratesio).

```rust
use canvasgen::{fingerprint_set, CanvasProfile};

let profile = CanvasProfile::new(42);
for canvas in fingerprint_set(&profile) {
    println!("{} {}x{} {} bytes", canvas.kind.as_str(), canvas.width, canvas.height, canvas.png.len());
    // canvas.png, canvas.rgba, canvas.to_data_url()
}
```

```bash
cargo build --release
./target/release/canvasgen probes --out samples --seed 42
./target/release/canvasgen url --kind triangle --seed 7
```

Or depend on the checkout:

```toml
canvasgen = { path = "../canvasgen_rs" }
```

## What it generates

The order and sizes mirror the captured hCaptcha probes:

| # | renderer | size | captured file | what it looks like |
|---|---|---|---|---|
| 0 | `small_probe` | 20×20 | `canvas_0_20x20.png` | colour emoji face |
| 1 | `gradient_triangle` | 300×150 | `canvas_1_300x150.png` | black field, RGB gradient triangle (WebGL probe) |
| 2 | `small_probe_with(Blob)` | 20×20 | `canvas_2_20x20.png` | transparent abstract thumbnail |
| 3 | `emoji_text` | 300×150 | `canvas_3_300x150.png` | emoji row (❤️🤪🎉👋) |
| 4 | `abstract_image` | 96×48 | `canvas_4_96x48.png` | translucent shapes/glyph runs on `#fa6939` |

`fingerprint_set(profile)` returns all five in that order.
`GeneratedCanvas` exposes `width`, `height`, straight-alpha `rgba`, `png`,
`data_url` and `to_data_url()` (`data:image/png;base64,...`).

The text probe uses the canvas default `10px` font, baseline 70, x=50 and a
transparent background, exactly like the capture. On Windows this reproduces
the captured row bounding box exactly (`(51, 60)…(102, 71)`); see
`samples/emoji_row_compare.png`, captured row on top and generated row below
(both composited on white for viewing).

## Full-colour emoji

The pipeline is deliberately the same stack a browser uses:

1. **Shaping** — `rustybuzz` shapes the run. ZWJ sequences, `VS15`/`VS16`,
   skin-tone modifiers and regional-indicator pairs become the font's own
   ligatures/composites. `shape()` returns glyph ids, advances and the number of
   input **clusters** (the stable signal for a single shaped unit).
2. **Scaling** —
   * `swash::scale` handles COLR/CPAL **v0** layered outlines, CBDT/CBLC and
     `sbix` colour bitmaps, compositing layers into RGBA.
   * COLR/CPAL **v1** paint graphs are traversed with `skrifa` (the font engine
     behind swash) and painted through `tiny-skia`, including nested clips,
     transforms, solid fills and linear/radial gradients.
3. **Compositing** — glyphs are blended source-over into the canvas, then
   encoded as a deterministic PNG.

### Support matrix

| format | fonts | status |
|---|---|---|
| COLR/CPAL v0 | Segoe UI Emoji (Windows), Twemoji Mozilla | ✅ verified (`samples/emoji_row_swash.png`) |
| COLR/CPAL v1 | newer Noto/Twemoji builds | ✅ verified with the fontations COLRv1 test font (`samples/colrv1_test_glyphs.png`); solid, linear and radial gradients, clips and transforms. Sweep gradients fall back to the first colour stop (tiny-skia has no sweep shader). |
| CBDT/CBLC | Noto Color Emoji | ✅ via swash (`StrikeWith::BestFit`) |
| sbix | Apple Color Emoji | ✅ via swash (bitmap extraction); not colour-tested on this Windows host |
| alpha bitmaps / outlines | any monochrome font | ✅ monochrome fallback |

“Colour first”: when a colour font is selected, the renderer prefers colour
outlines, then colour bitmaps, then outlines for glyphs that have no colour
representation. It only renders a monochrome run when the selected font has no
colour tables at all. If no font can be loaded, `emoji_text()` falls back to
built-in vector shapes so the probe still produces an image.

### Font discovery

`system_emoji_font()` uses `fontdb` and tries, in order:

* Windows: `Segoe UI Emoji`
* macOS: `Apple Color Emoji`
* Linux: `Noto Color Emoji`
* then `Twemoji Mozilla`, `EmojiOne Color`, `Noto Emoji`, and finally any system
  face whose family name contains “emoji” and which actually has a colour table.

Override with `--font PATH` (and `--font-index N` for TTC collections) or
`CanvasProfile::with_font_path`. No font is bundled or redistributed by this
crate.

### Font licensing

The crate ships **no font binaries**. System fonts remain governed by their own
licences (Segoe UI Emoji: Microsoft; Apple Color Emoji: Apple; Noto Color Emoji:
SIL OFL 1.1). If you pass a font with `--font`, you are responsible for its
licence. The optional sample rendered with Twemoji Mozilla
(`samples/emoji_*_twemoji.png`) uses Twemoji graphics (CC-BY 4.0) via the
Twemoji Mozilla font (Apache-2.0); the image is illustrative only and the font
is not included. The COLRv1 test uses `font-test-data` (MIT/Apache-2.0) as a
**dev-dependency** only.

### Chrome fidelity caveat

swash/zeno is not Skia. Glyph positions and advances match the captured probe
closely (the row bounding box matches exactly on Windows), but antialiasing,
hinting, bitmap-strike selection and layer rounding can differ by a pixel or a
few colour levels. PNG bytes are therefore **not** expected to equal Chrome’s
canvas output; the point is a plausible, deterministic, per-profile image.

Visual validation against the captures:

* **Emoji row** — same glyphs, same positions; bounding boxes match exactly
  (`(51, 60)…(102, 71)` at 300×150). The probe is transparent by default, like
  the capture; the saved samples composite it on white for easy viewing. Set
  `profile.background = Some([255,255,255,255])` for a white canvas.
* **Gradient triangle** — bounding box matches exactly
  (`(15, 37)…(269, 127)`), and the sampled RGB values match the captured
  formula `(2x/w, 2(h−y)/h, 255)`.
* **Small blob / abstract image** — same sizes, palette and shape language as
  the wasm program (transparent 20×20 thumbnail; `#fa6939` field, seven
  translucent rgba colours, polygons/arcs/ellipses/beziers/rects and
  monospace/cursive glyph runs for 96×48), but the composition is seed-driven,
  so it is a different image from the capture by design. The full extracted
  program and palette are documented in `spec/CANVAS_SPEC.md`.
  `samples/compare_captures.png` stacks capture (top) and generated (bottom)
  for all five.

Fidelity against a real browser (Chrome 148, Windows, RTX 4060, measured on
the capture machine; the Skia text style, `python`-free check in
`examples/fit_face.rs` / `examples/fit_ss.rs`):

| probe | before | now |
|---|---|---|
| 0 face 20x20 | mean 3.11, max 177 | mean 2.12, max 38 |
| 3 emoji row 300x150 | mean 0.12, max 135 | mean 0.03, max 54 |

The text probes are rendered at 8x (face) / 4x (row) and box-filtered back with
coverage-weighted colour, which approximates Skia's exact-area antialiasing much
better than a single 1x pass.  Probes 2 and 4 are seed-driven compositions, so
they are not comparable pixel-for-pixel.

Per-device GPU micro-variation: `SiliconProfile::from_seed` adds a sub-LSB
rounding bias to the WebGL triangle (before quantization). On the measured
Apple-vs-NVIDIA pair this produces 1-LSB contour flips on 3.9% of pixels; the
simulated variant lands at 4.5% with every delta exactly 1. Text uses
`TextStyle` (`Skia`/`Gecko`/`WebKit`): subpixel origin, alpha gamma and an
edge-weighted raster delta. Against the measured Chrome↔Firefox pair the row
lands at 0.84% / mean 0.27 (measured 0.84% / 0.28) and the 20×20 face at
40.5% / 4.65 (measured 38.5% / 4.4). `spec/DEVICE_VARIANCE.md` records the
calibration.

Two font-dependent behaviours are worth knowing:

* Flags (`🇺🇸`) only shape to one glyph when the font has the
  regional-indicator ligature. Apple Color Emoji, Noto Color Emoji and Twemoji
  do; **Segoe UI Emoji does not** (it renders `US`). `samples/emoji_flags_twemoji.png`
  shows real flags.
* Segoe composes a ZWJ family from several layered glyphs at one pen position.
  `shape()` reports **one cluster** (stable across fonts) even when the glyph
  count is greater than one. Tests assert cluster counts and advance widths,
  not a fixed glyph count.

## Determinism

The same seed + profile + platform/build produce byte-identical PNGs:

* `rand_chacha::ChaCha8Rng` seeded from `seed` and a per-probe salt
* fixed PNG settings (RGBA8, no row filter, default compression, no metadata)
* no timestamps, hash-map iteration or process RNG

Cross-platform floating-point differences (trigonometry in the abstract probe,
rasterizer rounding) can change individual pixels; determinism is guaranteed
per build/platform. `--dpr` scales the raster output and is part of the profile.

## CLI

```bash
# All five probes + manifest.json
canvasgen probes --out samples --seed 42

# One data URL
canvasgen url --kind text|triangle|small|abstract --seed 7

# The array env827 wants
canvasgen urls --seed 7 > canvasDataURLs.json

# Both arrays, ready to merge into env827.json
canvasgen env --seed 7 > canvas_env.json

# Show detected colour font / formats
canvasgen info
```

Useful flags: `--font PATH`, `--font-index N`, `--dpr 2`, and for `probes
--text` / `url --text`, `--font-size` and `--baseline`.

## Integrating with the hCaptcha harness

The hCaptcha JS installer implements `toDataURL()` as:

```js
const q = ENV.canvasDataURLs || [];
const i = (globalThis.__canvasIdx = (globalThis.__canvasIdx || 0) + 1) - 1;
return q[i % (q.length || 1)] || 'data:image/png;base64,';
```

Selection is therefore **deterministic by call index**, and the index must keep
the original sizes: 20×20, 300×150, 20×20, 300×150, 96×48. A misordered or
truncated capture list returns a 300×150 triangle where a 20×20 probe is
expected, which is an obvious tell. `fingerprint_set` preserves the index/size
mapping, and the generated URLs vary per profile while still being stable for a
given seed.

Both harnesses gained a `canvas` section:

```json
"canvas": { "mode": "generate", "count": 5, "seed": null, "font": null }
```

* `mode: "capture"` (default) keeps the existing `canvas_data_urls` /
  `canvas_ctxs` behaviour.
* `mode: "generate"` runs `canvasgen` (Python: `CANVASGEN_BIN`, `PATH`, or the
  local checkout binary; Rust: the `canvasgen` crate) and writes
  `canvasDataURLs`, plus the captured `canvasCtxs` when none are supplied.
* `seed: null` derives a stable seed from the profile (user agent, platform,
  hardware concurrency, screen, window), so one profile always gets one canvas
  set. Set an integer to pin it; `count` must be 5 for hCaptcha.
* `font` is a path passed through as `--font` (or loaded by the Rust hook).

See:

* `C:\Users\oli\Desktop\hcaptcha_research\epic_hcaptcha\fingerprint.py`
* `C:\Users\oli\Desktop\epic_hcaptcha_rs\src\canvas.rs`

## Samples

Generated by `cargo test --release` (see `tests/render.rs`):

| file | contents |
|---|---|
| `samples/emoji_row_swash.png` | captured emoji row, white background, Segoe COLRv0 |
| `samples/emoji_row_transparent.png` | same row, transparent background |
| `samples/emoji_row_compare.png` | captured row (top) vs generated row (bottom) |
| `samples/emoji_sequences.png` | ❤️ 👍🏽 👨‍👩‍👧‍👦 🇺🇸 with the system font |
| `samples/emoji_sequences_twemoji.png` | same set with Twemoji Mozilla (working flags) |
| `samples/emoji_flags_twemoji.png` | 🇺🇸 🇩🇪 🇯🇵 |
| `samples/colrv1_test_glyphs.png` | COLRv1 paint graphs: gradients, clips, circles |
| `samples/canvas_0_20x20.png` … `canvas_4_96x48.png` | the five probes |
| `samples/compare_captures.png` | all five: capture (top) vs generated (bottom) |

## Packaging status (GitHub source, not crates.io)

**The source is published at
https://github.com/oliking800/canvasgen_rs.** The crate is **not** uploaded to
crates.io; `cargo package` and `cargo publish --dry-run` are used only to verify
that the package builds from a clean tarball. The dry run aborts before upload.

```bash
cargo test --release
cargo package --allow-dirty        # tarball only
cargo publish --dry-run            # packaging + verify, no upload
```

The name `canvasgen` was checked on crates.io (`cargo search canvasgen`,
`cargo info canvasgen`) and is currently free. If a crates.io release is wanted
later, re-run the tests and confirm the font/licence notes first.

The package excludes `target/`, `captures/` and test fixtures; `font-test-data`
is a dev-dependency and is not pulled in by consumers. The crate is `MIT`
licensed; fonts are not included.

## Layout

```
src/lib.rs        public API and re-exports
src/profile.rs    CanvasProfile, GeneratedCanvas, ProbeKind
src/canvas.rs     tiny-skia canvas, alpha-correct blending, box resize
src/pngout.rs     deterministic PNG + base64 data URLs
src/emoji.rs      font discovery, rustybuzz shaping, swash/skrifa rasterization
src/probes.rs     text/triangle/small/abstract renderers + fingerprint_set
src/bin/canvasgen.rs  CLI
tests/render.rs   determinism, PNG/data-URL, colour, shaping, layout, samples
```
