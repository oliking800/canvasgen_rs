//! Canvas probe renderers.
//!
//! The mix and sizes mirror the five `canvasDataURLs` captured from hCaptcha:
//! `20x20` emoji face, `300x150` WebGL triangle, `20x20` abstract thumbnail,
//! `300x150` 2D emoji/text row, `96x48` seeded abstract image.

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use tiny_skia::{Paint, PathBuilder, Stroke, Transform};

use crate::canvas::{polygon_path, resize_box, Canvas};
use crate::emoji::{self, EmojiProfile};
use crate::profile::{CanvasProfile, GeneratedCanvas, ProbeKind};

/// Default row used by the text probe. This is the same sequence visible in
/// the captured `canvas_3_300x150.png`.
pub const DEFAULT_EMOJI_ROW: &str = "\u{2764}\u{FE0F}\u{1F600}\u{1F389}\u{1F446}";

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

/// Build the five URLs hCaptcha asks for, in the captured order.
pub fn fingerprint_set(profile: &CanvasProfile) -> Vec<GeneratedCanvas> {
    FINGERPRINT_PROBES
        .iter()
        .enumerate()
        .map(|(i, &(kind, w, h))| {
            let mut p = profile.clone();
            p.width = w;
            p.height = h;
            p.seed = mix_seed(profile.seed, i as u64);
            render_probe(kind, &p)
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

/// 2D text probe: the captured emoji row, using the canvas default `10px`
/// font, baseline 70, x=50. The captured canvas is transparent, so the default
/// background is transparent too; set `profile.background` for white.
pub fn emoji_text(profile: &CanvasProfile) -> GeneratedCanvas {
    let background = profile.background.unwrap_or([0, 0, 0, 0]);
    let emoji_profile = EmojiProfile {
        font_size: 10.0,
        dpr: profile.dpr,
        baseline: 70.0,
        letter_spacing: 0.0,
        antialias: profile.antialias,
    };
    emoji_row(profile, DEFAULT_EMOJI_ROW, &emoji_profile, background, 50.0)
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
        font_size: 10.0,
        dpr: profile.dpr,
        baseline: 70.0,
        letter_spacing: 0.0,
        antialias: profile.antialias,
    };
    emoji::render_fallback_row(&mut canvas, DEFAULT_EMOJI_ROW, &ep, 50.0, [32, 32, 32, 255]);
    GeneratedCanvas::from_canvas(ProbeKind::Text, &canvas)
}

// ---------------------------------------------------------------------------
// 2. Gradient triangle (WebGL probe)
// ---------------------------------------------------------------------------

/// Black field + a triangle whose colour is `(2x/w, 2(h-y)/h, 1)`.
pub fn gradient_triangle(profile: &CanvasProfile) -> GeneratedCanvas {
    let (w, h) = profile.pixel_size();
    let bg = profile.background.unwrap_or([0, 0, 0, 255]);
    let mut rgba = vec![0u8; w as usize * h as usize * 4];
    for px in rgba.chunks_exact_mut(4) {
        px.copy_from_slice(&bg);
    }

    let mut rng = rng_for(profile.seed, 0x7A11);
    let jitter_x = rng.gen_range(-0.012f32..0.012) * w as f32;
    let jitter_y = rng.gen_range(-0.010f32..0.010) * h as f32;
    let apex = (w as f32 * 0.50 + jitter_x, h as f32 * 0.242 + jitter_y);
    let left = (w as f32 * 0.050, h as f32 * 0.848);
    let right = (w as f32 * 0.898, h as f32 * 0.848);

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
            let r = (2.0 * cx / w as f32).clamp(0.0, 1.0) * 255.0;
            let g = (2.0 * (h as f32 - cy) / h as f32).clamp(0.0, 1.0) * 255.0;
            let src = [clamp_u8(r), clamp_u8(g), 255, 255];

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
    match kind {
        SmallKind::Emoji => {
            let (w, h) = profile.pixel_size();
            let mut canvas = Canvas::new(w, h);
            let text = "\u{1F603}";
            let font_size = profile.height as f32 * 0.78;
            let baseline = profile.height as f32 * 0.84;
            let mut ep = EmojiProfile {
                font_size,
                dpr: profile.dpr,
                baseline,
                letter_spacing: 0.0,
                antialias: profile.antialias,
            };
            ep.antialias &= profile.antialias;

            let drawn = profile
                .emoji_font()
                .ok()
                .and_then(|font| {
                    let adv = emoji::measure(text, &font, font_size, profile.dpr).ok()?;
                    let x = (profile.width as f32 - adv) * 0.5;
                    emoji::render_row(&mut canvas, &font, text, &ep, x, [32, 32, 32, 255]).ok()
                })
                .is_some();
            if !drawn {
                let x = profile.width as f32 * 0.5 - font_size * 0.5;
                emoji::render_fallback_row(&mut canvas, text, &ep, x, [32, 32, 32, 255]);
            }
            GeneratedCanvas::from_canvas(ProbeKind::Small, &canvas)
        }
        SmallKind::Blob => {
            let (w, h) = profile.pixel_size();
            let mut sub = profile.clone();
            sub.width = 96;
            sub.height = 48;
            sub.seed = mix_seed(profile.seed, 0xB10B);
            let big = abstract_image(&sub);
            let small = resize_box(&big.rgba, 96, 48, w, h);
            GeneratedCanvas::from_rgba(ProbeKind::SmallBlob, w, h, small)
        }
    }
}

// ---------------------------------------------------------------------------
// 4. Seeded abstract image
// ---------------------------------------------------------------------------

fn warm(rng: &mut ChaCha8Rng, base: [u8; 3], amount: i16) -> [u8; 4] {
    let mut channel =
        |v: u8| -> u8 { (v as i16 + rng.gen_range(-amount..=amount)).clamp(0, 255) as u8 };
    [channel(base[0]), channel(base[1]), channel(base[2]), 255]
}

/// Orange field, green star, dark-red strokes, a magenta wedge and seeded text.
pub fn abstract_image(profile: &CanvasProfile) -> GeneratedCanvas {
    let (w, h) = profile.pixel_size();
    let mut rng = rng_for(profile.seed, 0xA85);
    let bg = profile
        .background
        .unwrap_or_else(|| warm(&mut rng, [250, 105, 57], 6));
    let mut canvas = Canvas::filled(w, h, bg);
    let s = w as f32 / 96.0;

    // Two large translucent polygons.
    for i in 0..2 {
        let cx = rng.gen_range(0.05f32..0.95) * w as f32;
        let cy = rng.gen_range(0.15f32..0.85) * h as f32;
        let r = rng.gen_range(0.18f32..0.38) * w as f32;
        let n = 3 + (i * 2);
        let mut pts = Vec::new();
        for k in 0..n {
            let a = k as f32 / n as f32 * std::f32::consts::TAU + 0.4;
            pts.push((cx + a.cos() * r, cy + a.sin() * r * 0.8));
        }
        if let Some(path) = polygon_path(&pts) {
            canvas.fill_path(&path, [46, 120, 92, 150]);
        }
    }

    // Green star (10 vertices).
    let scx = 0.30 * w as f32;
    let scy = 0.50 * h as f32;
    let sr = 0.34 * w as f32;
    let rot = rng.gen_range(-0.8f32..-0.2);
    let mut star = Vec::new();
    for k in 0..10 {
        let a = rot + k as f32 / 10.0 * std::f32::consts::TAU;
        let r = if k % 2 == 0 { sr } else { sr * 0.42 };
        star.push((scx + a.cos() * r, scy + a.sin() * r));
    }
    if let Some(path) = polygon_path(&star) {
        canvas.fill_path(&path, [62, 170, 90, 242]);
    }

    // Dark red bezier strokes.
    let strokes = rng.gen_range(2..=4);
    for _ in 0..strokes {
        let mut pb = PathBuilder::new();
        let x0 = rng.gen_range(0.0f32..0.6) * w as f32;
        let y0 = rng.gen_range(0.2f32..0.9) * h as f32;
        let x3 = rng.gen_range(0.4f32..1.05) * w as f32;
        let y3 = rng.gen_range(0.1f32..0.9) * h as f32;
        pb.move_to(x0, y0);
        pb.cubic_to(
            rng.gen_range(0.0f32..1.0) * w as f32,
            rng.gen_range(0.0f32..1.0) * h as f32,
            rng.gen_range(0.0f32..1.0) * w as f32,
            rng.gen_range(0.0f32..1.0) * h as f32,
            x3,
            y3,
        );
        if let Some(path) = pb.finish() {
            let mut paint = Paint::default();
            paint.set_color_rgba8(142, 30, 77, 235);
            let stroke = Stroke {
                width: rng.gen_range(1.0f32..2.6) * s,
                ..Stroke::default()
            };
            canvas
                .pixmap_mut()
                .stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    // Magenta wedge on the right edge.
    let wx = w as f32 * rng.gen_range(0.78..0.95);
    let wy = h as f32 * rng.gen_range(0.1..0.4);
    if let Some(path) = polygon_path(&[
        (wx, wy),
        (wx + w as f32 * 0.10, wy + h as f32 * 0.12),
        (wx - w as f32 * 0.02, wy + h as f32 * 0.34),
    ]) {
        canvas.fill_path(&path, [214, 52, 112, 235]);
    }

    // Seeded text: real outlines when a font is available, bars otherwise.
    let words = ["Uproar", "Vqnt", "Pzor", "Quill", "Zephyr", "Nyx"];
    let text = words[rng.gen_range(0..words.len())];
    let ty = 10.0 * s;
    let tx = 2.0 * s;
    let font = profile.emoji_font().ok();
    let drew_text = font
        .as_ref()
        .map(|f| {
            let ep = EmojiProfile {
                font_size: 12.0 * s,
                dpr: 1.0,
                baseline: ty,
                letter_spacing: 0.0,
                antialias: profile.antialias,
            };
            emoji::render_row(&mut canvas, f, text, &ep, tx, [155, 191, 59, 255]).is_ok()
        })
        .unwrap_or(false);
    if !drew_text {
        for i in 0..6 {
            let bx = 2.0 * s + i as f32 * 7.0 * s + rng.gen_range(0.0..1.5) * s;
            let bw = rng.gen_range(3.0..6.0) * s;
            let by = (2.0 + rng.gen_range(0.0..3.0)) * s;
            canvas.fill_rect(bx, by, bw, 1.6 * s, [155, 191, 59, 220]);
        }
    }

    GeneratedCanvas::from_canvas(ProbeKind::Abstract, &canvas)
}

// ---------------------------------------------------------------------------
// Helpers used by callers that build their own layouts
// ---------------------------------------------------------------------------

/// Convenience: 2D context style rectangle fill on a fresh transparent canvas.
pub fn blank_canvas(width: u32, height: u32) -> Canvas {
    Canvas::new(width, height)
}
