//! Canvas probe renderers.
//!
//! The mix and sizes mirror the five `canvasDataURLs` captured from hCaptcha:
//! `20x20` emoji face, `300x150` WebGL triangle, `20x20` abstract thumbnail,
//! `300x150` 2D emoji/text row, `96x48` seeded abstract image.

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use tiny_skia::PathBuilder;

use crate::canvas::{polygon_path, Canvas};
use crate::emoji::{self, EmojiProfile, TextFamily};
use crate::profile::{CanvasProfile, GeneratedCanvas, ProbeKind, SiliconProfile, TextStyle};

/// Default row used by the text probe. This is the exact sequence emitted by
/// the hCaptcha build (`canvas` op `fillText("Ã¢ÂÂ¤Ã¯Â¸ÂÃ°Å¸Â¤ÂªÃ°Å¸Å½â€°Ã°Å¸â€˜â€¹", 50, 70)`).
pub const DEFAULT_EMOJI_ROW: &str = "\u{2764}\u{FE0F}\u{1F92A}\u{1F389}\u{1F44B}";

/// `fillText` origin and baseline for the text probe.
pub const TEXT_ROW_X: f32 = 50.0;
pub const TEXT_ROW_BASELINE: f32 = 70.0;

/// Canvas-default font size used by the captured `fillText` op.
pub const TEXT_ROW_FONT_SIZE: f32 = 10.0;

/// Sizes used by [`fingerprint_set`], matching the capture order.
pub const FINGERPRINT_PROBES: [(ProbeKind, u32, u32); 5] = [
    (ProbeKind::Small, 20, 20),
    (ProbeKind::Triangle, 300, 150),
    (ProbeKind::SmallBlob, 20, 20),
    (ProbeKind::Text, 300, 150),
    (ProbeKind::Abstract, 96, 48),
];

fn rng_for(seed: u64, salt: u64) -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(seed ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

fn mix_seed(seed: u64, index: u64) -> u64 {
    let mut x = seed ^ index.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

fn clamp_u8(v: f32) -> u8 {
    v.clamp(0.0, 255.0).round() as u8
}

/// Apply an alpha-only gamma to straight-alpha RGBA (text edge intensity).
fn apply_alpha_gamma(rgba: &mut [u8], gamma: f32) {
    if (gamma - 1.0).abs() < 1e-4 {
        return;
    }
    let mut lut = [0u8; 256];
    for (i, slot) in lut.iter_mut().enumerate() {
        *slot = (255.0 * (i as f32 / 255.0).powf(gamma)).round() as u8;
    }
    for px in rgba.chunks_exact_mut(4) {
        px[3] = lut[px[3] as usize];
    }
}

/// Deterministic edge-weighted delta that stands in for another engine's
/// colour-glyph rasterizer. Real Gecko/WebKit output differs from Skia mostly
/// on glyph edges (measured: 38.5 % of a 20x20 face, mean 4.4, max 51; 0.84 %
/// of a 300x150 row). Flat areas get a small base delta, edges the full
/// amplitude.
pub(crate) fn perturb_text_edges(rgba: &mut [u8], w: u32, h: u32, amplitude: f32, salt: u32) {
    if amplitude <= 0.0 || w < 3 || h < 3 {
        return;
    }
    let (wi, hi) = (w as isize, h as isize);
    let alpha: Vec<u8> = rgba.chunks_exact(4).map(|p| p[3]).collect();
    let unit = |x: isize, y: isize| -> f32 {
        let mut v =
            (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77) ^ salt;
        v ^= v >> 15;
        v = v.wrapping_mul(0x2C1B_3C6D);
        v ^= v >> 12;
        v = v.wrapping_mul(0x297A_2D39);
        v ^= v >> 15;
        (v as f32 / u32::MAX as f32) * 2.0 - 1.0
    };
    for y in 1..hi - 1 {
        for x in 1..wi - 1 {
            let i = (y * wi + x) as usize;
            let gx = (alpha[i + 1] as i32 - alpha[i - 1] as i32).abs();
            let gy = (alpha[i + wi as usize] as i32 - alpha[i - wi as usize] as i32).abs();
            let grad = (gx + gy).min(255) as f32 / 255.0;
            if alpha[i] == 0 && grad == 0.0 {
                continue;
            }
            let weight = 0.015 + 0.985 * grad;
            let d = unit(x, y) * amplitude * 255.0 * weight;
            let px = &mut rgba[i * 4..i * 4 + 4];
            for c in 0..3 {
                px[c] = (px[c] as f32 + d).clamp(0.0, 255.0).round() as u8;
            }
            px[3] = (px[3] as f32 + d * 0.5).clamp(0.0, 255.0).round() as u8;
        }
    }
}

/// Straight-alpha source-over.
fn blend_rgba(dst: [u8; 4], src: [u8; 4], coverage: f32) -> [u8; 4] {
    let a = (src[3] as f32 / 255.0) * coverage.clamp(0.0, 1.0);
    if a <= 0.0 {
        return dst;
    }
    let da = dst[3] as f32 / 255.0;
    let oa = a + da * (1.0 - a);
    if oa <= 0.0 {
        return [0, 0, 0, 0];
    }
    let mix = |s: u8, d: u8| ((s as f32 * a + d as f32 * da * (1.0 - a)) / oa).clamp(0.0, 255.0);
    [
        clamp_u8(mix(src[0], dst[0])),
        clamp_u8(mix(src[1], dst[1])),
        clamp_u8(mix(src[2], dst[2])),
        clamp_u8(oa * 255.0),
    ]
}

/// Three-pass box blur of the alpha channel, returned as a separate mask.
/// Kept for callers that need canvas `shadowBlur` emulation.
#[allow(dead_code)]
fn blur_alpha(rgba: &[u8], w: u32, h: u32, blur: f32) -> Vec<u8> {
    let (w, h) = (w as usize, h as usize);
    let sigma = (blur * 0.5).max(0.0);
    let radius = (sigma * 1.5).round().max(0.0) as isize;
    if radius == 0 || w == 0 || h == 0 {
        return rgba.chunks_exact(4).map(|p| p[3]).collect();
    }

    let mut a: Vec<f32> = rgba.chunks_exact(4).map(|p| p[3] as f32).collect();
    let mut tmp = vec![0.0f32; a.len()];
    let norm = 1.0 / (2 * radius + 1) as f32;

    for _ in 0..3 {
        // horizontal
        for y in 0..h {
            let row = y * w;
            let mut acc = 0.0;
            for x in -radius..=radius {
                let xi = x.clamp(0, w as isize - 1) as usize;
                acc += a[row + xi];
            }
            for x in 0..w {
                tmp[row + x] = acc * norm;
                let out = (x as isize - radius).clamp(0, w as isize - 1) as usize;
                let inp = (x as isize + radius + 1).clamp(0, w as isize - 1) as usize;
                acc += a[row + inp] - a[row + out];
            }
        }
        // vertical
        for x in 0..w {
            let mut acc = 0.0;
            for y in -radius..=radius {
                let yi = y.clamp(0, h as isize - 1) as usize;
                acc += tmp[yi * w + x];
            }
            for y in 0..h {
                a[y * w + x] = acc * norm;
                let out = (y as isize - radius).clamp(0, h as isize - 1) as usize;
                let inp = (y as isize + radius + 1).clamp(0, h as isize - 1) as usize;
                acc += tmp[inp * w + x] - tmp[out * w + x];
            }
        }
    }

    a.into_iter().map(|v| v.clamp(0.0, 255.0) as u8).collect()
}

/// Build a straight-alpha shadow image from the alpha channel of `rgba`.
#[allow(dead_code)]
fn shadow_from(rgba: &[u8], w: u32, h: u32, blur: f32, color: [u8; 4]) -> Vec<u8> {
    let alpha = blur_alpha(rgba, w, h, blur);
    let mut out = vec![0u8; alpha.len() * 4];
    for (i, a) in alpha.into_iter().enumerate() {
        let a = (a as u32 * color[3] as u32 / 255) as u8;
        out[i * 4] = color[0];
        out[i * 4 + 1] = color[1];
        out[i * 4 + 2] = color[2];
        out[i * 4 + 3] = a;
    }
    out
}

/// Renders any probe kind.
pub fn render_probe(kind: ProbeKind, profile: &CanvasProfile) -> GeneratedCanvas {
    match kind {
        ProbeKind::Text => emoji_text(profile),
        ProbeKind::Triangle => gradient_triangle(profile),
        ProbeKind::Small => small_probe(profile),
        ProbeKind::SmallBlob => small_probe_with(profile, SmallKind::Blob),
        ProbeKind::Abstract => abstract_image(profile),
    }
}

/// Build the five URLs hCaptcha asks for, in the captured order, using a
/// per-device text style derived from the profile seed.
pub fn fingerprint_set(profile: &CanvasProfile) -> Vec<GeneratedCanvas> {
    fingerprint_set_with(profile, TextStyle::from_seed(profile.seed))
}

/// Five probes with an explicit text style.
pub fn fingerprint_set_with(profile: &CanvasProfile, style: TextStyle) -> Vec<GeneratedCanvas> {
    FINGERPRINT_PROBES
        .iter()
        .enumerate()
        .map(|(i, &(kind, w, h))| {
            let mut p = profile.clone();
            p.width = w;
            p.height = h;
            p.seed = mix_seed(profile.seed, i as u64);
            match kind {
                ProbeKind::Text => emoji_text_with(&p, style),
                ProbeKind::Small => small_probe_with_style(&p, SmallKind::Emoji, style),
                _ => render_probe(kind, &p),
            }
        })
        .collect()
}

/// Just the data URLs, ready for `env.canvasDataURLs`.
pub fn canvas_data_urls(profile: &CanvasProfile) -> Vec<String> {
    fingerprint_set(profile)
        .into_iter()
        .map(|c| c.data_url)
        .collect()
}

/// Data URLs with an explicit text style (e.g. derived from the claimed UA).
pub fn canvas_data_urls_with(profile: &CanvasProfile, style: TextStyle) -> Vec<String> {
    fingerprint_set_with(profile, style)
        .into_iter()
        .map(|c| c.data_url)
        .collect()
}

/// The context list captured alongside the URLs (for `env.canvasCtxs`).
pub const CANVAS_CTXS_JSON: &str = concat!(
    "[{\"label\":\"offscreen\",\"type\":\"webgl2\",\"ok\":true,\"w\":1,\"h\":1},",
    "{\"label\":\"html\",\"type\":\"2d\",\"ok\":true,\"w\":300,\"h\":150},",
    "{\"label\":\"html\",\"type\":\"webgl\",\"ok\":true,\"w\":300,\"h\":150},",
    "{\"label\":\"html\",\"type\":\"2d\",\"ok\":true,\"w\":300,\"h\":150},",
    "{\"label\":\"html\",\"type\":\"2d\",\"ok\":true,\"w\":300,\"h\":150},",
    "{\"label\":\"html\",\"type\":\"2d\",\"ok\":true,\"w\":300,\"h\":150}]"
);

// ---------------------------------------------------------------------------
// 1. Text / emoji row
// ---------------------------------------------------------------------------

/// 2D text probe: the captured emoji row. The recorded browser op is
/// `fillText("Ã¢ÂÂ¤Ã¯Â¸ÂÃ°Å¸Â¤ÂªÃ°Å¸Å½â€°Ã°Å¸â€˜â€¹", 50, 70)` in the canvas default `10px sans-serif`
/// with `#000000` (colour emoji ignore the fill). The captured canvas is
/// transparent, so the default background is transparent too; set
/// `profile.background` for white.
pub fn emoji_text(profile: &CanvasProfile) -> GeneratedCanvas {
    emoji_text_with(profile, TextStyle::neutral())
}

/// Text probe with a device text style (subpixel origin, alpha gamma).
pub fn emoji_text_with(profile: &CanvasProfile, style: TextStyle) -> GeneratedCanvas {
    let (w, h) = profile.pixel_size();
    let background = profile.background.unwrap_or([0, 0, 0, 0]);
    let mut canvas = Canvas::filled(w, h, background);
    let mut text = Canvas::new(w, h);
    let ep = EmojiProfile {
        font_size: TEXT_ROW_FONT_SIZE,
        dpr: profile.dpr,
        baseline: TEXT_ROW_BASELINE + style.dy,
        letter_spacing: 0.0,
        antialias: profile.antialias,
    };
    let x = TEXT_ROW_X + style.dx;
    match profile.emoji_font() {
        Ok(font) => {
            if emoji::render_row(
                &mut text,
                &font,
                DEFAULT_EMOJI_ROW,
                &ep,
                x,
                [32, 32, 32, 255],
            )
            .is_err()
            {
                emoji::render_fallback_row(&mut text, DEFAULT_EMOJI_ROW, &ep, x, [32, 32, 32, 255]);
            }
        }
        Err(_) => {
            emoji::render_fallback_row(&mut text, DEFAULT_EMOJI_ROW, &ep, x, [32, 32, 32, 255]);
        }
    }
    let mut rgba = text.to_rgba();
    apply_alpha_gamma(&mut rgba, style.gamma);
    perturb_text_edges(&mut rgba, w, h, style.raster_delta, style.salt);
    canvas.blend_image(0, 0, w, h, &rgba);
    GeneratedCanvas::from_canvas(ProbeKind::Text, &canvas)
}

/// Render an arbitrary emoji/text row. Falls back to vector shapes when no font
/// can be loaded, so the probe always produces an image.
pub fn emoji_row(
    profile: &CanvasProfile,
    text: &str,
    emoji_profile: &EmojiProfile,
    background: [u8; 4],
    x: f32,
) -> GeneratedCanvas {
    let (w, h) = profile.pixel_size();
    let mut canvas = Canvas::filled(w, h, background);
    let mut ep = emoji_profile.clone();
    ep.antialias &= profile.antialias;

    match profile.emoji_font() {
        Ok(font) => {
            if emoji::render_row(&mut canvas, &font, text, &ep, x, [32, 32, 32, 255]).is_err() {
                emoji::render_fallback_row(&mut canvas, text, &ep, x, [32, 32, 32, 255]);
            }
        }
        Err(_) => {
            emoji::render_fallback_row(&mut canvas, text, &ep, x, [32, 32, 32, 255]);
        }
    }
    GeneratedCanvas::from_canvas(ProbeKind::Text, &canvas)
}

/// Exposed for tests / callers that explicitly want the vector fallback.
pub fn emoji_text_fallback(profile: &CanvasProfile) -> GeneratedCanvas {
    let (w, h) = profile.pixel_size();
    let mut canvas = Canvas::filled(w, h, profile.background.unwrap_or([0, 0, 0, 0]));
    let ep = EmojiProfile {
        font_size: TEXT_ROW_FONT_SIZE,
        dpr: profile.dpr,
        baseline: TEXT_ROW_BASELINE,
        letter_spacing: 0.0,
        antialias: profile.antialias,
    };
    emoji::render_fallback_row(
        &mut canvas,
        DEFAULT_EMOJI_ROW,
        &ep,
        TEXT_ROW_X,
        [32, 32, 32, 255],
    );
    GeneratedCanvas::from_canvas(ProbeKind::Text, &canvas)
}

// ---------------------------------------------------------------------------
// 2. Gradient triangle (WebGL probe)
// ---------------------------------------------------------------------------

/// Black field + a triangle whose colour is `(2x/w, 2(h-y)/h, 1)`.
///
/// Uses the device's sub-LSB rounding bias ([`SiliconProfile::from_seed`]).
pub fn gradient_triangle(profile: &CanvasProfile) -> GeneratedCanvas {
    gradient_triangle_with(profile, SiliconProfile::from_seed(profile.seed))
}

/// Gradient triangle with an explicit rasterizer bias.
pub fn gradient_triangle_with(profile: &CanvasProfile, silicon: SiliconProfile) -> GeneratedCanvas {
    let (w, h) = profile.pixel_size();
    let bg = profile.background.unwrap_or([0, 0, 0, 255]);
    let mut rgba = vec![0u8; w as usize * h as usize * 4];
    for px in rgba.chunks_exact_mut(4) {
        px.copy_from_slice(&bg);
    }

    let apex = (w as f32 * 0.50, h as f32 * 0.25);
    let left = (w as f32 * 0.05, h as f32 * 0.85);
    let right = (w as f32 * 0.90, h as f32 * 0.85);

    let edge = |a: (f32, f32), b: (f32, f32), p: (f32, f32)| {
        (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0)
    };
    let e0 = edge(apex, left, right);
    let inside = |p: (f32, f32)| {
        let e1 = edge(apex, left, p);
        let e2 = edge(left, right, p);
        let e3 = edge(right, apex, p);
        if e0 >= 0.0 {
            e1 >= 0.0 && e2 >= 0.0 && e3 >= 0.0
        } else {
            e1 <= 0.0 && e2 <= 0.0 && e3 <= 0.0
        }
    };

    let aa = profile.antialias;
    let samples = if aa { 4 } else { 1 };
    for y in 0..h {
        for x in 0..w {
            let mut hits = 0u32;
            if aa {
                for sy in 0..samples {
                    for sx in 0..samples {
                        let px = x as f32 + (sx as f32 + 0.5) / samples as f32;
                        let py = y as f32 + (sy as f32 + 0.5) / samples as f32;
                        if inside((px, py)) {
                            hits += 1;
                        }
                    }
                }
            } else if inside((x as f32 + 0.5, y as f32 + 0.5)) {
                hits = 1;
            }
            if hits == 0 {
                continue;
            }
            let coverage = hits as f32 / (samples * samples) as f32;

            let cx = x as f32 + 0.5;
            let cy = y as f32 + 0.5;
            let r = (2.0 * cx / w as f32).clamp(0.0, 1.0) * 255.0 + silicon.rgb_bias[0];
            let g =
                (2.0 * (h as f32 - cy) / h as f32).clamp(0.0, 1.0) * 255.0 + silicon.rgb_bias[1];
            let b = 255.0 + silicon.rgb_bias[2];
            let src = [clamp_u8(r), clamp_u8(g), clamp_u8(b), 255];

            let i = (y as usize * w as usize + x as usize) * 4;
            let dst = [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]];
            let out = blend_rgba(dst, src, coverage);
            rgba[i..i + 4].copy_from_slice(&out);
        }
    }

    GeneratedCanvas::from_rgba(ProbeKind::Triangle, w, h, rgba)
}

// ---------------------------------------------------------------------------
// 3. Small probes
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmallKind {
    /// Smiling emoji face (`canvas_0`).
    Emoji,
    /// Blurred abstract thumbnail (`canvas_2`).
    Blob,
}

/// 20x20 emoji face probe.
pub fn small_probe(profile: &CanvasProfile) -> GeneratedCanvas {
    small_probe_with(profile, SmallKind::Emoji)
}

pub fn small_probe_with(profile: &CanvasProfile, kind: SmallKind) -> GeneratedCanvas {
    small_probe_with_style(profile, kind, TextStyle::neutral())
}

/// Small probe with a device text style.
pub fn small_probe_with_style(
    profile: &CanvasProfile,
    kind: SmallKind,
    style: TextStyle,
) -> GeneratedCanvas {
    match kind {
        SmallKind::Emoji => {
            let (w, h) = profile.pixel_size();
            let mut canvas = Canvas::new(w, h);
            let mut text = Canvas::new(w, h);
            let text_str = "\u{1F600}";
            let mut ep = EmojiProfile {
                font_size: 15.0,
                dpr: profile.dpr,
                baseline: 15.0 + style.dy,
                letter_spacing: 0.0,
                antialias: profile.antialias,
            };
            ep.antialias &= profile.antialias;

            let drawn = profile
                .emoji_font()
                .ok()
                .and_then(|font| {
                    emoji::render_row(&mut text, &font, text_str, &ep, style.dx, [0, 0, 0, 255])
                        .ok()
                })
                .is_some();
            if !drawn {
                emoji::render_fallback_row(&mut text, text_str, &ep, style.dx, [0, 0, 0, 255]);
            }
            let mut rgba = text.to_rgba();
            apply_alpha_gamma(&mut rgba, style.gamma);
            perturb_text_edges(&mut rgba, w, h, style.raster_delta, style.salt);
            canvas.blend_image(0, 0, w, h, &rgba);

            // The captured URL-0 face is the emoji only. The extra
            // fillRect/arc tail in the op list belongs to the sixth canvas
            // (index 5), whose data URL the installer never reads.
            GeneratedCanvas::from_canvas(ProbeKind::Small, &canvas)
        }
        SmallKind::Blob => {
            let (w, h) = profile.pixel_size();
            let mut sub = profile.clone();
            sub.width = w;
            sub.height = h;
            // The wasm draws the abstract program directly at 20x20 on a
            // transparent canvas (no background fillRect).
            sub.background = Some([0, 0, 0, 0]);
            sub.seed = mix_seed(profile.seed, 0xB10B);
            let mut c = abstract_image(&sub);
            c.kind = ProbeKind::SmallBlob;
            c
        }
    }
}

// ---------------------------------------------------------------------------
// 4. Seeded abstract image
// ---------------------------------------------------------------------------

/// Background and translucent fill/stroke palette from the wasm program
/// (`#fa6939` field; `rgba(207,2,220,0.38)`, `rgba(38,136,74,0.74)`, ...).
pub const ABSTRACT_BG: [u8; 4] = [0xFA, 0x69, 0x39, 255];
pub const ABSTRACT_FILLS: [[u8; 4]; 7] = [
    [207, 2, 220, 97],
    [33, 85, 92, 94],
    [38, 136, 74, 189],
    [131, 14, 80, 212],
    [73, 120, 88, 196],
    [100, 141, 191, 191],
    [89, 248, 7, 161],
];
const ABSTRACT_STROKES: [[u8; 4]; 4] = [
    [131, 14, 80, 212],
    [33, 85, 92, 94],
    [73, 120, 88, 196],
    [89, 248, 7, 161],
];
const ABSTRACT_LINE_WIDTHS: [f32; 6] = [3.0, 1.125, 1.1875, 0.5625, 2.8125, 2.0];
const ABSTRACT_TEXT_SIZES: [f32; 4] = [12.75, 18.06, 8.12, 14.25];

fn ellipse_path(cx: f32, cy: f32, rx: f32, ry: f32, rot: f32) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    let steps = 32;
    for i in 0..=steps {
        let t = i as f32 / steps as f32 * std::f32::consts::TAU;
        let (x, y) = (rx * t.cos(), ry * t.sin());
        let (x, y) = (
            cx + x * rot.cos() - y * rot.sin(),
            cy + x * rot.sin() + y * rot.cos(),
        );
        if i == 0 {
            pb.move_to(x, y);
        } else {
            pb.line_to(x, y);
        }
    }
    pb.close();
    pb.finish()
}

/// Seeded glyph string: the wasm uses random Latin/Greek/Cyrillic/CJK glyphs.
fn noise_text(rng: &mut ChaCha8Rng, len: usize) -> String {
    const RANGES: [(u32, u32); 5] = [
        (0x00C0, 0x00FF),
        (0x0370, 0x03FF),
        (0x0400, 0x04FF),
        (0x4E00, 0x9FFF),
        (0x2500, 0x257F),
    ];
    let mut s = String::new();
    for _ in 0..len {
        let (lo, hi) = RANGES[rng.gen_range(0..RANGES.len())];
        let cp = rng.gen_range(lo..=hi);
        if let Some(ch) = char::from_u32(cp) {
            s.push(ch);
        }
    }
    s
}

/// Orange field with the wasm's shape mix: translucent polygons, arcs,
/// ellipses, bezier strokes, rectangles and random glyph runs.
pub fn abstract_image(profile: &CanvasProfile) -> GeneratedCanvas {
    let (w, h) = profile.pixel_size();
    let mut rng = rng_for(profile.seed, 0xA85);
    let bg = profile.background.unwrap_or(ABSTRACT_BG);
    let mut canvas = Canvas::filled(w, h, bg);
    let s = (w as f32 / 96.0).min(h as f32 / 48.0);
    let wf = w as f32;
    let hf = h as f32;

    // Translucent polygons (3-4 vertices), like the captured program.
    for i in 0..7 {
        let n = if i % 3 == 0 { 4 } else { 3 };
        let pts: Vec<(f32, f32)> = (0..n)
            .map(|_| {
                (
                    rng.gen_range(0.0f32..1.0) * wf,
                    rng.gen_range(0.0f32..1.0) * hf,
                )
            })
            .collect();
        if let Some(path) = polygon_path(&pts) {
            canvas.fill_path(&path, ABSTRACT_FILLS[i % ABSTRACT_FILLS.len()]);
        }
    }

    // Arcs / ellipses: fill and stroke, cycling through the palette.
    for i in 0..4 {
        let cx = rng.gen_range(0.0f32..1.0) * wf;
        let cy = rng.gen_range(0.0f32..1.0) * hf;
        let rx = rng.gen_range(0.05f32..0.25) * wf;
        let ry = rng.gen_range(0.05f32..0.25) * wf;
        let a0 = rng.gen_range(0.0f32..std::f32::consts::TAU);
        let a1 = a0 + rng.gen_range(0.5f32..std::f32::consts::TAU);
        let path = if i % 2 == 0 {
            sector_path(cx, cy, rx.max(ry), a0, a1)
        } else {
            ellipse_path(cx, cy, rx, ry, rng.gen_range(0.0f32..std::f32::consts::TAU))
        };
        if let Some(path) = path {
            canvas.fill_path(&path, ABSTRACT_FILLS[(i + 2) % ABSTRACT_FILLS.len()]);
            let lw = ABSTRACT_LINE_WIDTHS[i % ABSTRACT_LINE_WIDTHS.len()] * s;
            canvas.stroke_path(&path, ABSTRACT_STROKES[i % ABSTRACT_STROKES.len()], lw);
        }
    }

    // Bezier strokes.
    for _ in 0..3 {
        let mut pb = PathBuilder::new();
        pb.move_to(
            rng.gen_range(0.0f32..1.0) * wf,
            rng.gen_range(0.0f32..1.0) * hf,
        );
        pb.cubic_to(
            rng.gen_range(0.0f32..1.0) * wf,
            rng.gen_range(0.0f32..1.0) * hf,
            rng.gen_range(0.0f32..1.0) * wf,
            rng.gen_range(0.0f32..1.0) * hf,
            rng.gen_range(0.0f32..1.0) * wf,
            rng.gen_range(0.0f32..1.0) * hf,
        );
        if let Some(path) = pb.finish() {
            let lw = ABSTRACT_LINE_WIDTHS[rng.gen_range(0..ABSTRACT_LINE_WIDTHS.len())] * s;
            canvas.stroke_path(&path, ABSTRACT_STROKES[1], lw);
        }
    }

    // Two rectangles.
    for _ in 0..2 {
        let x = rng.gen_range(0.0f32..0.9) * wf;
        let y = rng.gen_range(0.0f32..0.9) * hf;
        let rw = rng.gen_range(0.05f32..0.25) * wf;
        let rh = rng.gen_range(0.05f32..0.25) * hf;
        canvas.fill_rect(
            x,
            y,
            rw,
            rh,
            ABSTRACT_FILLS[rng.gen_range(0..ABSTRACT_FILLS.len())],
        );
    }

    // Two glyph runs, matching the wasm's `strokeText` + `fillText` pairs.
    for i in 0..2 {
        let n = rng.gen_range(4..=8);
        let text = noise_text(&mut rng, n);
        let family = if i % 2 == 0 {
            TextFamily::Monospace
        } else {
            TextFamily::Cursive
        };
        let size = ABSTRACT_TEXT_SIZES[rng.gen_range(0..ABSTRACT_TEXT_SIZES.len())] * s;
        let tx = rng.gen_range(0.0f32..0.7) * wf;
        let ty = rng.gen_range(0.2f32..0.95) * hf;
        if let Ok(font) = crate::emoji::system_text_font(family) {
            let ep = EmojiProfile {
                font_size: size.max(1.0),
                dpr: 1.0,
                baseline: ty,
                letter_spacing: 0.0,
                antialias: profile.antialias,
            };
            let stroke = ABSTRACT_STROKES[i % ABSTRACT_STROKES.len()];
            let _ = emoji::render_row(&mut canvas, &font, &text, &ep, tx - 0.5 * s, stroke);
            let _ = emoji::render_row(&mut canvas, &font, &text, &ep, tx, ABSTRACT_FILLS[i]);
        }
    }

    GeneratedCanvas::from_canvas(ProbeKind::Abstract, &canvas)
}

// ---------------------------------------------------------------------------
// Helpers used by callers that build their own layouts
// ---------------------------------------------------------------------------

/// Pie-slice path used by the 20x20 probe's `arc(...)` fill.
fn sector_path(cx: f32, cy: f32, r: f32, a0: f32, a1: f32) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    pb.move_to(cx, cy);
    let steps = 24;
    for i in 0..=steps {
        let a = a0 + (a1 - a0) * i as f32 / steps as f32;
        pb.line_to(cx + a.cos() * r, cy + a.sin() * r);
    }
    pb.close();
    pb.finish()
}

/// Convenience: 2D context style rectangle fill on a fresh transparent canvas.
pub fn blank_canvas(width: u32, height: u32) -> Canvas {
    Canvas::new(width, height)
}
