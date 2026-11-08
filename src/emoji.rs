//! Full-colour emoji: font discovery, shaping and glyph rasterization.
//!
//! * [`rustybuzz`] shapes the run so ZWJ sequences, flags, skin-tone modifiers
//!   and `VS15`/`VS16` collapse into the glyphs the font actually provides.
//! * [`swash`] scales COLR/CPAL v0 layered outlines and CBDT/CBLC + sbix colour
//!   bitmaps into RGBA images.
//! * COLR/CPAL v1 paint graphs are traversed with [`skrifa`] (which backs
//!   swash's font machinery) and painted through `tiny-skia`.
//!
//! No font is bundled. `--font PATH` overrides system discovery; the font's
//! licence stays with the font.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::FontRef as SwashFont;

use crate::canvas::{circle_path, polygon_path, Canvas};

/// Colour-table flavour available in a font.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorSupport {
    /// No colour tables; glyphs rasterize to an alpha mask.
    None,
    /// COLR/CPAL v0 (layered glyph outlines; Segoe UI Emoji on Windows 10/11).
    ColrV0,
    /// COLR/CPAL v1 (paint graph; newer emoji fonts).
    ColrV1,
    /// CBDT/CBLC embedded PNG bitmaps (Noto Color Emoji).
    Cbdt,
    /// Apple `sbix` embedded bitmaps (Apple Color Emoji).
    Sbix,
}

impl ColorSupport {
    pub fn is_color(self) -> bool {
        !matches!(self, ColorSupport::None)
    }

    pub fn label(self) -> &'static str {
        match self {
            ColorSupport::None => "monochrome",
            ColorSupport::ColrV0 => "COLRv0/CPAL",
            ColorSupport::ColrV1 => "COLRv1/CPAL",
            ColorSupport::Cbdt => "CBDT/CBLC",
            ColorSupport::Sbix => "sbix",
        }
    }
}

/// A loaded emoji (or fallback) font.
#[derive(Clone, Debug)]
pub struct EmojiFont {
    pub data: Arc<Vec<u8>>,
    pub face_index: u32,
    pub family: String,
    pub color: ColorSupport,
    pub path: Option<std::path::PathBuf>,
}

impl EmojiFont {
    pub fn from_bytes(
        data: Vec<u8>,
        face_index: u32,
        family: impl Into<String>,
    ) -> Result<Self, String> {
        let color = detect_color(&data, face_index)?;
        // Validate that swash can parse it too.
        SwashFont::from_index(&data, face_index as usize)
            .ok_or_else(|| "font could not be parsed".to_string())?;
        Ok(Self {
            data: Arc::new(data),
            face_index,
            family: family.into(),
            color,
            path: None,
        })
    }

    pub fn from_file(path: &Path, face_index: u32) -> Result<Self, String> {
        let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let family = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "embedded".into());
        let mut font = Self::from_bytes(data, face_index, family)?;
        font.path = Some(path.to_path_buf());
        Ok(font)
    }

    /// True when the font has any colour table (COLR, CBDT/CBLC or sbix).
    pub fn has_color(&self) -> bool {
        self.color.is_color()
    }
}

/// Rendering parameters for a text/emoji row (logical CSS pixels; multiplied
/// by `dpr` before rasterization).
#[derive(Clone, Debug)]
pub struct EmojiProfile {
    pub font_size: f32,
    pub dpr: f32,
    pub baseline: f32,
    pub letter_spacing: f32,
    pub antialias: bool,
}

impl Default for EmojiProfile {
    /// Matches the captured probe: the canvas default `10px` font, baseline 70.
    fn default() -> Self {
        Self {
            font_size: 10.0,
            dpr: 1.0,
            baseline: 70.0,
            letter_spacing: 0.0,
            antialias: true,
        }
    }
}

/// One shaped glyph. Advances are in font units; use [`ShapedRun::scale`] to
/// convert to pixels.
#[derive(Clone, Debug)]
pub struct ShapedGlyph {
    pub glyph_id: u16,
    pub cluster: u32,
    pub x_advance: f32,
    pub y_advance: f32,
    pub x_offset: f32,
    pub y_offset: f32,
}

/// Result of shaping a run.
#[derive(Clone, Debug)]
pub struct ShapedRun {
    pub glyphs: Vec<ShapedGlyph>,
    pub units_per_em: f32,
    /// Number of distinct input clusters. A ZWJ family, a flag or a skin-tone
    /// sequence normally yields one cluster.
    pub clusters: usize,
    pub text_len: usize,
}

impl ShapedRun {
    pub fn scale(&self, px: f32) -> f32 {
        if self.units_per_em > 0.0 {
            px / self.units_per_em
        } else {
            1.0
        }
    }

    /// Total advance in pixels at the given font size.
    pub fn advance_px(&self, px: f32) -> f32 {
        self.glyphs.iter().map(|g| g.x_advance).sum::<f32>() * self.scale(px)
    }
}

/// Statistics returned from [`render_row`].
#[derive(Clone, Copy, Debug, Default)]
pub struct RowStats {
    pub glyphs: usize,
    pub clusters: usize,
    pub advance: f32,
    pub color_glyphs: usize,
    pub mask_glyphs: usize,
}

/// Shape `text` with `font`.
pub fn shape(text: &str, font: &EmojiFont) -> Result<ShapedRun, String> {
    let face = rustybuzz::Face::from_slice(&font.data, font.face_index)
        .ok_or_else(|| "rustybuzz: font parse failed".to_string())?;
    let upem = face.units_per_em() as f32;
    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_direction(rustybuzz::Direction::LeftToRight);
    let out = rustybuzz::shape(&face, &[], buffer);

    let mut glyphs = Vec::with_capacity(out.glyph_infos().len());
    let mut clusters = BTreeSet::new();
    for (info, pos) in out.glyph_infos().iter().zip(out.glyph_positions().iter()) {
        clusters.insert(info.cluster);
        glyphs.push(ShapedGlyph {
            glyph_id: info.glyph_id as u16,
            cluster: info.cluster,
            x_advance: pos.x_advance as f32,
            y_advance: pos.y_advance as f32,
            x_offset: pos.x_offset as f32,
            y_offset: pos.y_offset as f32,
        });
    }
    Ok(ShapedRun {
        glyphs,
        units_per_em: if upem > 0.0 { upem } else { 1000.0 },
        clusters: clusters.len(),
        text_len: text.len(),
    })
}

/// Total advance of `text` in logical pixels.
pub fn measure(text: &str, font: &EmojiFont, font_size: f32, dpr: f32) -> Result<f32, String> {
    let run = shape(text, font)?;
    Ok(run.advance_px(font_size * dpr) / dpr)
}

/// Render `text` at `origin_x` (logical px) / `profile.baseline` into `canvas`.
///
/// `foreground` is used for glyphs that only have an alpha mask. Colour glyphs
/// keep their palette colours.
pub fn render_row(
    canvas: &mut Canvas,
    font: &EmojiFont,
    text: &str,
    profile: &EmojiProfile,
    origin_x: f32,
    foreground: [u8; 4],
) -> Result<RowStats, String> {
    let run = shape(text, font)?;
    let px = (profile.font_size * profile.dpr).max(1.0);
    let scale = run.scale(px);
    let baseline = profile.baseline * profile.dpr;
    let mut pen = origin_x * profile.dpr;

    let swash_font = SwashFont::from_index(&font.data, font.face_index as usize)
        .ok_or_else(|| "swash: font parse failed".to_string())?;
    let mut ctx = ScaleContext::new();
    let mut scaler = ctx.builder(swash_font).size(px).hint(false).build();

    let color_sources = [
        Source::ColorOutline(0),
        Source::ColorBitmap(StrikeWith::BestFit),
        Source::Outline,
    ];
    let mono_sources = [Source::Outline];
    let renderer = if font.color.is_color() {
        Render::new(&color_sources)
    } else {
        Render::new(&mono_sources)
    };

    let mut stats = RowStats {
        glyphs: run.glyphs.len(),
        clusters: run.clusters,
        ..Default::default()
    };

    for glyph in &run.glyphs {
        let gx = pen + glyph.x_offset * scale;
        let gy = baseline - glyph.y_offset * scale;

        // A glyph id of 0 is `.notdef`. Browsers fall back to another font for
        // spaces / missing ASCII; drawing the tofu box would add pixels the
        // captured probe never has. Skip it and keep a sensible advance for
        // whitespace.
        let is_whitespace = text
            .get(glyph.cluster as usize..)
            .and_then(|s| s.chars().next())
            .map(|c| c.is_whitespace())
            .unwrap_or(false);

        if glyph.glyph_id != 0 {
            match draw_glyph(
                canvas,
                font,
                &mut scaler,
                &renderer,
                glyph.glyph_id,
                gx,
                gy,
                px,
                profile.antialias,
                foreground,
            )? {
                Some(Drawn::Color) => stats.color_glyphs += 1,
                Some(Drawn::Mask) => stats.mask_glyphs += 1,
                None => {}
            }
        }

        let mut advance = glyph.x_advance * scale;
        if advance == 0.0 && is_whitespace {
            advance = px * 0.33;
        }
        pen += advance + profile.letter_spacing * profile.dpr;
    }

    stats.advance = pen / profile.dpr;
    Ok(stats)
}

/// Render a single glyph by id. Returns `Ok(true)` when a colour glyph was
/// drawn, `Ok(false)` for a mask/outline glyph, and `Err` only for a broken
/// font/paint graph. Useful for fonts without a cmap (test fonts, icon fonts).
#[allow(clippy::too_many_arguments)]
pub fn render_glyph(
    canvas: &mut Canvas,
    font: &EmojiFont,
    glyph_id: u16,
    px: f32,
    pen_x: f32,
    baseline: f32,
    antialias: bool,
    foreground: [u8; 4],
) -> Result<bool, String> {
    let swash_font = SwashFont::from_index(&font.data, font.face_index as usize)
        .ok_or_else(|| "swash: font parse failed".to_string())?;
    let mut ctx = ScaleContext::new();
    let mut scaler = ctx.builder(swash_font).size(px).hint(false).build();
    let color_sources = [
        Source::ColorOutline(0),
        Source::ColorBitmap(StrikeWith::BestFit),
        Source::Outline,
    ];
    let mono_sources = [Source::Outline];
    let renderer = if font.color.is_color() {
        Render::new(&color_sources)
    } else {
        Render::new(&mono_sources)
    };
    match draw_glyph(
        canvas,
        font,
        &mut scaler,
        &renderer,
        glyph_id,
        pen_x,
        baseline,
        px,
        antialias,
        foreground,
    )? {
        Some(Drawn::Color) => Ok(true),
        _ => Ok(false),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Drawn {
    Color,
    Mask,
}

#[allow(clippy::too_many_arguments)]
fn draw_glyph(
    canvas: &mut Canvas,
    font: &EmojiFont,
    scaler: &mut swash::scale::Scaler<'_>,
    renderer: &Render<'_>,
    glyph_id: u16,
    gx: f32,
    gy: f32,
    px: f32,
    antialias: bool,
    foreground: [u8; 4],
) -> Result<Option<Drawn>, String> {
    if font.color == ColorSupport::ColrV1 {
        if let Some(true) =
            render_colrv1(canvas, font, glyph_id, px, gx, gy, antialias, foreground)?
        {
            return Ok(Some(Drawn::Color));
        }
    }

    if let Some(image) = renderer.render(scaler, glyph_id) {
        let dest_x = (gx + image.placement.left as f32).round() as i32;
        let dest_y = (gy - image.placement.top as f32).round() as i32;
        match image.content {
            swash::scale::image::Content::Color => {
                canvas.blend_image(
                    dest_x,
                    dest_y,
                    image.placement.width,
                    image.placement.height,
                    &image.data,
                );
                return Ok(Some(Drawn::Color));
            }
            swash::scale::image::Content::Mask => {
                blit_mask(
                    canvas,
                    dest_x,
                    dest_y,
                    image.placement.width,
                    image.placement.height,
                    &image.data,
                    foreground,
                    antialias,
                );
                return Ok(Some(Drawn::Mask));
            }
            swash::scale::image::Content::SubpixelMask => {
                blit_subpixel(
                    canvas,
                    dest_x,
                    dest_y,
                    image.placement.width,
                    image.placement.height,
                    &image.data,
                    foreground,
                    antialias,
                );
                return Ok(Some(Drawn::Mask));
            }
        }
    }
    Ok(None)
}

fn blit_mask(
    canvas: &mut Canvas,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    mask: &[u8],
    color: [u8; 4],
    antialias: bool,
) {
    for row in 0..h as i32 {
        for col in 0..w as i32 {
            let i = (row as usize) * (w as usize) + col as usize;
            let Some(&m) = mask.get(i) else { continue };
            let a = if antialias {
                (m as u32 * color[3] as u32 / 255) as u8
            } else if m >= 128 {
                color[3]
            } else {
                0
            };
            if a > 0 {
                canvas.blend_pixel(x + col, y + row, [color[0], color[1], color[2], a]);
            }
        }
    }
}

fn blit_subpixel(
    canvas: &mut Canvas,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    data: &[u8],
    color: [u8; 4],
    antialias: bool,
) {
    for row in 0..h as i32 {
        for col in 0..w as i32 {
            let i = ((row as usize) * (w as usize) + col as usize) * 4;
            let Some(px) = data.get(i..i + 4) else {
                continue;
            };
            let m = ((px[0] as u32 + px[1] as u32 + px[2] as u32) / 3) as u8;
            let a = if antialias {
                (m as u32 * color[3] as u32 / 255) as u8
            } else if m >= 128 {
                color[3]
            } else {
                0
            };
            if a > 0 {
                canvas.blend_pixel(x + col, y + row, [color[0], color[1], color[2], a]);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// COLRv1 painting (via skrifa, the font engine behind swash)
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn render_colrv1(
    canvas: &mut Canvas,
    font: &EmojiFont,
    glyph_id: u16,
    px: f32,
    pen_x: f32,
    baseline: f32,
    antialias: bool,
    foreground: [u8; 4],
) -> Result<Option<bool>, String> {
    use skrifa::raw::TableProvider as _;
    use skrifa::MetadataProvider as _;

    let font_ref = skrifa::FontRef::from_index(&font.data, font.face_index as u32)
        .map_err(|e| format!("skrifa: {e}"))?;
    let upem = font_ref
        .head()
        .map(|h| h.units_per_em() as f32)
        .unwrap_or(1000.0);
    let scale = if upem > 0.0 { px / upem } else { 1.0 };

    let glyphs = font_ref.color_glyphs();
    let Some(color_glyph) = glyphs.get_with_format(
        skrifa::GlyphId::new(glyph_id as u32),
        skrifa::color::ColorGlyphFormat::ColrV1,
    ) else {
        return Ok(None);
    };

    let palette = font_ref
        .color_palettes()
        .get(0)
        .map(|p| {
            p.colors()
                .iter()
                .map(|c| [c.red(), c.green(), c.blue(), c.alpha()])
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let base = tiny_skia::Transform::from_row(scale, 0.0, 0.0, -scale, pen_x, baseline);
    let outlines = font_ref.outline_glyphs();
    let mut painter = ColrPainter {
        canvas,
        base,
        transforms: vec![base],
        clips: vec![None],
        palette,
        foreground,
        outlines,
        antialias,
        painted: false,
    };
    color_glyph
        .paint(skrifa::instance::LocationRef::default(), &mut painter)
        .map_err(|e| format!("colrv1 paint: {e}"))?;
    Ok(Some(painter.painted))
}

struct ColrPainter<'a> {
    canvas: &'a mut Canvas,
    base: tiny_skia::Transform,
    transforms: Vec<tiny_skia::Transform>,
    clips: Vec<Option<tiny_skia::Mask>>,
    palette: Vec<[u8; 4]>,
    foreground: [u8; 4],
    outlines: skrifa::outline::OutlineGlyphCollection<'a>,
    antialias: bool,
    painted: bool,
}

impl ColrPainter<'_> {
    fn current(&self) -> tiny_skia::Transform {
        *self.transforms.last().unwrap_or(&self.base)
    }

    fn current_clip(&self) -> Option<&tiny_skia::Mask> {
        self.clips.last().and_then(|c| c.as_ref())
    }

    fn palette_color(&self, index: u16, alpha: f32) -> [u8; 4] {
        let c = self
            .palette
            .get(index as usize)
            .copied()
            .unwrap_or(self.foreground);
        let a = ((c[3] as f32 / 255.0) * alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
        [c[0], c[1], c[2], a]
    }

    fn push_mask(&mut self, mask: tiny_skia::Mask) {
        let next = match self.clips.last().and_then(|c| c.as_ref()) {
            Some(prev) => intersect_masks(prev, &mask),
            None => mask,
        };
        self.clips.push(Some(next));
    }
}

impl skrifa::color::ColorPainter for ColrPainter<'_> {
    fn push_transform(&mut self, transform: skrifa::color::Transform) {
        let t = tiny_skia::Transform::from_row(
            transform.xx,
            transform.yx,
            transform.xy,
            transform.yy,
            transform.dx,
            transform.dy,
        );
        let next = self.current().pre_concat(t);
        self.transforms.push(next);
    }

    fn pop_transform(&mut self) {
        if self.transforms.len() > 1 {
            self.transforms.pop();
        }
    }

    fn push_clip_glyph(&mut self, glyph_id: skrifa::GlyphId) {
        let Some(glyph) = self.outlines.get(glyph_id) else {
            return;
        };
        let mut pen = PathPen::default();
        if glyph
            .draw(
                skrifa::outline::DrawSettings::unhinted(
                    skrifa::instance::Size::unscaled(),
                    skrifa::instance::LocationRef::default(),
                ),
                &mut pen,
            )
            .is_err()
        {
            return;
        }
        let Some(path) = pen.finish() else { return };
        let Some(mut mask) = tiny_skia::Mask::new(self.canvas.width(), self.canvas.height()) else {
            return;
        };
        mask.fill_path(
            &path,
            tiny_skia::FillRule::Winding,
            self.antialias,
            self.current(),
        );
        self.push_mask(mask);
    }

    fn push_clip_box(&mut self, clip_box: skrifa::raw::types::BoundingBox<f32>) {
        let Some(rect) = tiny_skia::Rect::from_ltrb(
            clip_box.x_min,
            clip_box.y_min,
            clip_box.x_max,
            clip_box.y_max,
        ) else {
            return;
        };
        let path = tiny_skia::PathBuilder::from_rect(rect);
        let Some(mut mask) = tiny_skia::Mask::new(self.canvas.width(), self.canvas.height()) else {
            return;
        };
        mask.fill_path(
            &path,
            tiny_skia::FillRule::Winding,
            self.antialias,
            self.current(),
        );
        self.push_mask(mask);
    }

    fn pop_clip(&mut self) {
        if self.clips.len() > 1 {
            self.clips.pop();
        }
    }

    fn fill(&mut self, brush: skrifa::color::Brush<'_>) {
        let ts = self.current();
        let Some(rect) = tiny_skia::Rect::from_xywh(
            0.0,
            0.0,
            self.canvas.width() as f32,
            self.canvas.height() as f32,
        ) else {
            return;
        };
        let mut paint = tiny_skia::Paint::default();
        paint.anti_alias = self.antialias;

        match brush {
            skrifa::color::Brush::Solid {
                palette_index,
                alpha,
            } => {
                let c = self.palette_color(palette_index, alpha);
                paint.set_color_rgba8(c[0], c[1], c[2], c[3]);
            }
            skrifa::color::Brush::LinearGradient {
                p0,
                p1,
                color_stops,
                extend,
            } => {
                let stops = gradient_stops(color_stops, self);
                let Some(shader) = tiny_skia::LinearGradient::new(
                    tiny_skia::Point::from_xy(p0.x, p0.y),
                    tiny_skia::Point::from_xy(p1.x, p1.y),
                    stops,
                    spread_mode(extend),
                    ts,
                ) else {
                    return;
                };
                paint.shader = shader;
            }
            skrifa::color::Brush::RadialGradient {
                c0,
                r0,
                c1,
                r1,
                color_stops,
                extend,
            } => {
                let stops = gradient_stops(color_stops, self);
                let Some(shader) = tiny_skia::RadialGradient::new(
                    tiny_skia::Point::from_xy(c0.x, c0.y),
                    r0,
                    tiny_skia::Point::from_xy(c1.x, c1.y),
                    r1,
                    stops,
                    spread_mode(extend),
                    ts,
                ) else {
                    return;
                };
                paint.shader = shader;
            }
            skrifa::color::Brush::SweepGradient { .. } => {
                // tiny-skia has no sweep shader; use the first stop colour.
                // Sweep gradients are rare in emoji fonts.
                let c = self.palette.first().copied().unwrap_or(self.foreground);
                paint.set_color_rgba8(c[0], c[1], c[2], c[3]);
            }
        }

        let mask = self.current_clip().cloned();
        self.canvas.pixmap_mut().fill_rect(
            rect,
            &paint,
            tiny_skia::Transform::identity(),
            mask.as_ref(),
        );
        self.painted = true;
    }

    fn push_layer(&mut self, _composite_mode: skrifa::color::CompositeMode) {
        // Emoji paint graphs use layers mostly for shadows/highlights. Drawing
        // them source-over is a close approximation and keeps the implementation
        // deterministic.
    }

    fn pop_layer(&mut self) {}
}

fn intersect_masks(a: &tiny_skia::Mask, b: &tiny_skia::Mask) -> tiny_skia::Mask {
    let (w, h) = (a.width(), a.height());
    let mut out = tiny_skia::Mask::new(w, h).expect("mask");
    for i in 0..(w as usize * h as usize) {
        let v = (a.data()[i] as u32 * b.data()[i] as u32 + 127) / 255;
        out.data_mut()[i] = v as u8;
    }
    out
}

fn gradient_stops(
    stops: &[skrifa::color::ColorStop],
    painter: &ColrPainter<'_>,
) -> Vec<tiny_skia::GradientStop> {
    stops
        .iter()
        .map(|s| {
            let c = painter.palette_color(s.palette_index, s.alpha);
            tiny_skia::GradientStop::new(
                s.offset.clamp(0.0, 1.0),
                tiny_skia::Color::from_rgba8(c[0], c[1], c[2], c[3]),
            )
        })
        .collect()
}

fn spread_mode(extend: skrifa::color::Extend) -> tiny_skia::SpreadMode {
    match extend {
        skrifa::color::Extend::Repeat => tiny_skia::SpreadMode::Repeat,
        skrifa::color::Extend::Reflect => tiny_skia::SpreadMode::Reflect,
        _ => tiny_skia::SpreadMode::Pad,
    }
}

/// Builds a `tiny-skia` path from skrifa outline commands.
#[derive(Default)]
struct PathPen {
    builder: Option<tiny_skia::PathBuilder>,
}

impl PathPen {
    fn finish(self) -> Option<tiny_skia::Path> {
        self.builder.and_then(|b| b.finish())
    }
}

impl skrifa::outline::OutlinePen for PathPen {
    fn move_to(&mut self, x: f32, y: f32) {
        if self.builder.is_none() {
            self.builder = Some(tiny_skia::PathBuilder::new());
        }
        if let Some(b) = &mut self.builder {
            b.move_to(x, y);
        }
    }

    fn line_to(&mut self, x: f32, y: f32) {
        if let Some(b) = &mut self.builder {
            b.line_to(x, y);
        }
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        if let Some(b) = &mut self.builder {
            b.quad_to(cx0, cy0, x, y);
        }
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        if let Some(b) = &mut self.builder {
            b.cubic_to(cx0, cy0, cx1, cy1, x, y);
        }
    }

    fn close(&mut self) {
        if let Some(b) = &mut self.builder {
            b.close();
        }
    }
}

// ---------------------------------------------------------------------------
// Font discovery
// ---------------------------------------------------------------------------

/// Return the platform colour emoji font, cached for the process.
pub fn system_emoji_font() -> Result<EmojiFont, String> {
    static CACHE: OnceLock<Result<EmojiFont, String>> = OnceLock::new();
    CACHE.get_or_init(load_system_emoji_font).clone()
}

fn candidate_families() -> Vec<&'static str> {
    #[cfg(target_os = "windows")]
    let first = "Segoe UI Emoji";
    #[cfg(target_os = "macos")]
    let first = "Apple Color Emoji";
    #[cfg(all(unix, not(target_os = "macos")))]
    let first = "Noto Color Emoji";
    #[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
    let first = "Noto Color Emoji";

    let mut v = vec![
        first,
        "Segoe UI Emoji",
        "Apple Color Emoji",
        "Noto Color Emoji",
        "Twemoji Mozilla",
        "EmojiOne Color",
        "Noto Emoji",
    ];
    v.dedup();
    v
}

fn load_system_emoji_font() -> Result<EmojiFont, String> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    for family in candidate_families() {
        let query = fontdb::Query {
            families: &[fontdb::Family::Name(family)],
            ..fontdb::Query::default()
        };
        if let Some(id) = db.query(&query) {
            if let Some((data, index)) = db.with_face_data(id, |data, index| (data.to_vec(), index))
            {
                if let Ok(font) = EmojiFont::from_bytes(data, index, family) {
                    return Ok(font);
                }
            }
        }
    }

    // Last resort: any face whose family name mentions "emoji" (localised names
    // included) and which actually has a colour table.
    let mut fallback: Option<(fontdb::ID, String)> = None;
    for face in db.faces() {
        let name = face
            .families
            .iter()
            .map(|(n, _)| n)
            .find(|n| n.to_ascii_lowercase().contains("emoji"))
            .cloned();
        if let Some(name) = name {
            fallback.get_or_insert((face.id, name));
        }
    }
    if let Some((id, name)) = fallback {
        if let Some((data, index)) = db.with_face_data(id, |data, index| (data.to_vec(), index)) {
            if let Ok(font) = EmojiFont::from_bytes(data, index, &name) {
                return Ok(font);
            }
        }
    }

    Err(
        "no system emoji font found (tried Segoe UI Emoji, Apple Color Emoji, \
         Noto Color Emoji); pass --font PATH"
            .to_string(),
    )
}

fn detect_color(data: &[u8], face_index: u32) -> Result<ColorSupport, String> {
    use skrifa::raw::TableProvider as _;

    let font =
        skrifa::FontRef::from_index(data, face_index).map_err(|e| format!("font parse: {e}"))?;
    if let Ok(colr) = font.colr() {
        return Ok(if colr.version() >= 1 {
            ColorSupport::ColrV1
        } else {
            ColorSupport::ColrV0
        });
    }
    if font.cbdt().is_ok() {
        return Ok(ColorSupport::Cbdt);
    }
    if font.sbix().is_ok() {
        return Ok(ColorSupport::Sbix);
    }
    Ok(ColorSupport::None)
}

/// Human-readable description of the system emoji font (CLI `info`).
pub fn system_font_info() -> String {
    match system_emoji_font() {
        Ok(f) => format!(
            "{} ({}, face {}, {})",
            f.family,
            f.color.label(),
            f.face_index,
            f.path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "system".into())
        ),
        Err(e) => e,
    }
}

// ---------------------------------------------------------------------------
// Shape-based fallback (no font available)
// ---------------------------------------------------------------------------

/// Draw the row with coloured vector shapes when no font is available.
///
/// This keeps the crate usable on headless machines and in `no font` tests; it
/// is intentionally a rough approximation, not a substitute for real emoji.
pub fn render_fallback_row(
    canvas: &mut Canvas,
    text: &str,
    profile: &EmojiProfile,
    origin_x: f32,
    _foreground: [u8; 4],
) -> RowStats {
    let em = (profile.font_size * profile.dpr).max(1.0);
    let baseline = profile.baseline * profile.dpr;
    let mut pen = origin_x * profile.dpr;
    let clusters = simple_clusters(text);
    for cluster in &clusters {
        let cp = cluster.chars().next().map(|c| c as u32).unwrap_or(0);
        draw_emoji_shape(canvas, cp, pen, baseline, em);
        pen += em + profile.letter_spacing * profile.dpr;
    }
    RowStats {
        glyphs: clusters.len(),
        clusters: clusters.len(),
        advance: pen / profile.dpr,
        color_glyphs: clusters.len(),
        mask_glyphs: 0,
    }
}

fn simple_clusters(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut prev_zwj = false;
    let mut prev_ri = false;
    for ch in text.chars() {
        let cp = ch as u32;
        let vi = is_variation(cp);
        let combining = is_combining(cp);
        let skin = (0x1F3FB..=0x1F3FF).contains(&cp);
        let ri = (0x1F1E6..=0x1F1FF).contains(&cp);
        let zwj = cp == 0x200D;
        let extends = vi || combining || skin || zwj || prev_zwj || (ri && prev_ri);
        if !extends && !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
        cur.push(ch);
        prev_zwj = zwj;
        prev_ri = ri;
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn is_variation(cp: u32) -> bool {
    cp == 0xFE0E || cp == 0xFE0F
}

fn is_combining(cp: u32) -> bool {
    matches!(cp, 0x0300..=0x036F | 0x1AB0..=0x1AFF | 0x20D0..=0x20FF)
}

fn draw_emoji_shape(canvas: &mut Canvas, cp: u32, x: f32, baseline: f32, em: f32) {
    let cx = x + em * 0.5;
    let cy = baseline - em * 0.5;
    match cp {
        0x2764 | 0x2665 => {
            // Heart
            let r = em * 0.22;
            let left = circle_path(cx - r * 0.85, cy - r * 0.9, r);
            let right = circle_path(cx + r * 0.85, cy - r * 0.9, r);
            if let Some(p) = left {
                canvas.fill_path(&p, [218, 30, 52, 255]);
            }
            if let Some(p) = right {
                canvas.fill_path(&p, [218, 30, 52, 255]);
            }
            if let Some(p) = polygon_path(&[
                (cx - em * 0.42, cy - em * 0.13),
                (cx + em * 0.42, cy - em * 0.13),
                (cx, cy + em * 0.42),
            ]) {
                canvas.fill_path(&p, [218, 30, 52, 255]);
            }
        }
        0x1F600..=0x1F64F => {
            // Smiling face
            if let Some(p) = circle_path(cx, cy, em * 0.44) {
                canvas.fill_path(&p, [255, 200, 61, 255]);
            }
            for dx in [-1.0f32, 1.0] {
                if let Some(p) = circle_path(cx + dx * em * 0.16, cy - em * 0.10, em * 0.055) {
                    canvas.fill_path(&p, [30, 24, 12, 255]);
                }
            }
            if let Some(p) = ellipse_path(cx, cy + em * 0.13, em * 0.20, em * 0.13) {
                canvas.fill_path(&p, [120, 24, 24, 255]);
            }
        }
        0x1F389 => {
            // Party popper: cone + confetti
            if let Some(p) = polygon_path(&[
                (cx - em * 0.40, cy + em * 0.40),
                (cx + em * 0.10, cy + em * 0.18),
                (cx - em * 0.02, cy - em * 0.10),
                (cx - em * 0.48, cy + em * 0.24),
            ]) {
                canvas.fill_path(&p, [244, 190, 48, 255]);
            }
            let dots = [
                (0.25f32, -0.38f32, [46, 120, 210, 255]),
                (0.40, -0.18, [200, 40, 90, 255]),
                (0.16, -0.42, [70, 160, 80, 255]),
            ];
            for (dx, dy, col) in dots {
                if let Some(p) = circle_path(cx + dx * em, cy + dy * em, em * 0.07) {
                    canvas.fill_path(&p, col);
                }
            }
        }
        0x1F446 | 0x1F447 | 0x1F448 | 0x1F449 => {
            // Pointing hand
            if let Some(p) = rounded_rect(
                cx - em * 0.12,
                cy - em * 0.34,
                em * 0.24,
                em * 0.58,
                em * 0.1,
            ) {
                canvas.fill_path(&p, [255, 200, 61, 255]);
            }
            if let Some(p) = polygon_path(&[
                (cx - em * 0.08, cy - em * 0.34),
                (cx + em * 0.08, cy - em * 0.34),
                (cx, cy - em * 0.52),
            ]) {
                canvas.fill_path(&p, [255, 200, 61, 255]);
            }
            canvas.fill_rect(
                cx - em * 0.20,
                cy + em * 0.18,
                em * 0.40,
                em * 0.16,
                [40, 70, 150, 255],
            );
        }
        0x1F535 => {
            if let Some(p) = circle_path(cx, cy, em * 0.42) {
                canvas.fill_path(&p, [42, 90, 220, 255]);
            }
        }
        _ => {
            let hue = (cp.wrapping_mul(2654435761) % 360) as f32 / 360.0;
            let col = hue_rgb(hue);
            if let Some(p) = rounded_rect(
                cx - em * 0.36,
                cy - em * 0.36,
                em * 0.72,
                em * 0.72,
                em * 0.16,
            ) {
                canvas.fill_path(&p, [col[0], col[1], col[2], 255]);
            }
        }
    }
}

fn ellipse_path(cx: f32, cy: f32, rx: f32, ry: f32) -> Option<tiny_skia::Path> {
    let mut pb = tiny_skia::PathBuilder::new();
    pb.push_oval(tiny_skia::Rect::from_xywh(
        cx - rx,
        cy - ry,
        rx * 2.0,
        ry * 2.0,
    )?);
    pb.finish()
}

fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<tiny_skia::Path> {
    let r = r.min(w * 0.5).min(h * 0.5).max(0.0);
    let mut pb = tiny_skia::PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish()
}

fn hue_rgb(h: f32) -> [u8; 3] {
    let h = (h.fract() * 6.0).abs();
    let x = (1.0 - (h % 2.0 - 1.0).abs()).clamp(0.0, 1.0);
    let (r, g, b) = match h as u32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    [
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    ]
}
