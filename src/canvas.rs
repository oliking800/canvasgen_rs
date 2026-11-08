//! Small wrapper around `tiny-skia` that keeps straight-alpha RGBA output and
//! offers the few primitives the canvas probes need.

use tiny_skia::{
    Color, FillRule, Mask, Paint, Path, PathBuilder, Pixmap, PremultipliedColorU8, Rect, Stroke,
    Transform,
};

/// A raster canvas with premultiplied pixels internally and straight-alpha
/// output (the layout used by `CanvasRenderingContext2D.getImageData`).
pub struct Canvas {
    pixmap: Pixmap,
}

impl Canvas {
    /// Allocate a transparent canvas. Degenerate/oversized dimensions are
    /// clamped to `1..=4096` per axis; allocation failure falls back to 1x1.
    pub fn new(width: u32, height: u32) -> Self {
        let w = width.clamp(1, crate::profile::MAX_CANVAS_DIM);
        let h = height.clamp(1, crate::profile::MAX_CANVAS_DIM);
        let pixmap = Pixmap::new(w, h)
            .or_else(|| Pixmap::new(1, 1))
            .expect("1x1 pixmap");
        Self { pixmap }
    }

    /// Allocate and fill a canvas with `color`.
    pub fn filled(width: u32, height: u32, color: [u8; 4]) -> Self {
        let mut c = Self::new(width, height);
        c.clear(color);
        c
    }

    /// Raster width in pixels.
    pub fn width(&self) -> u32 {
        self.pixmap.width()
    }

    /// Raster height in pixels.
    pub fn height(&self) -> u32 {
        self.pixmap.height()
    }

    /// Borrow the underlying tiny-skia pixmap.
    pub fn pixmap(&self) -> &Pixmap {
        &self.pixmap
    }

    /// Mutably borrow the underlying tiny-skia pixmap.
    pub fn pixmap_mut(&mut self) -> &mut Pixmap {
        &mut self.pixmap
    }

    /// Replace every pixel with `color` (straight alpha).
    pub fn clear(&mut self, color: [u8; 4]) {
        self.pixmap
            .fill(Color::from_rgba8(color[0], color[1], color[2], color[3]));
    }

    /// Straight-alpha RGBA bytes, top-left origin, `width * height * 4` long.
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.pixmap.data().len());
        for px in self.pixmap.pixels() {
            let c = px.demultiply();
            out.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
        }
        out
    }

    /// Fill an axis-aligned rectangle with a straight-alpha colour.
    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [u8; 4]) {
        let Some(rect) = Rect::from_xywh(x, y, w, h) else {
            return;
        };
        let mut paint = Paint::default();
        paint.set_color_rgba8(color[0], color[1], color[2], color[3]);
        self.pixmap
            .fill_rect(rect, &paint, Transform::identity(), None);
    }

    /// Fill a path with a straight-alpha colour.
    pub fn fill_path(&mut self, path: &Path, color: [u8; 4]) {
        let mut paint = Paint::default();
        paint.set_color_rgba8(color[0], color[1], color[2], color[3]);
        self.pixmap
            .fill_path(path, &paint, FillRule::Winding, Transform::identity(), None);
    }

    /// Fill a path with a custom paint, transform and optional clip mask.
    pub fn fill_path_with(
        &mut self,
        path: &Path,
        paint: &Paint,
        transform: Transform,
        mask: Option<&Mask>,
    ) {
        self.pixmap
            .fill_path(path, paint, FillRule::Winding, transform, mask);
    }

    /// Stroke a path with a straight-alpha colour.
    pub fn stroke_path(&mut self, path: &Path, color: [u8; 4], width: f32) {
        let mut paint = Paint::default();
        paint.set_color_rgba8(color[0], color[1], color[2], color[3]);
        let stroke = Stroke {
            width,
            ..Stroke::default()
        };
        self.pixmap
            .stroke_path(path, &paint, &stroke, Transform::identity(), None);
    }

    #[allow(clippy::too_many_arguments)]
    /// Stroke a path with a transform and optional clip mask.
    pub fn stroke_path_with(
        &mut self,
        path: &Path,
        color: [u8; 4],
        width: f32,
        transform: Transform,
        mask: Option<&Mask>,
    ) {
        let mut paint = Paint::default();
        paint.set_color_rgba8(color[0], color[1], color[2], color[3]);
        let stroke = Stroke {
            width,
            ..Stroke::default()
        };
        self.pixmap
            .stroke_path(path, &paint, &stroke, transform, mask);
    }

    /// Source-over blend of one straight-alpha pixel into the canvas.
    /// Source-over a single straight-alpha pixel at `(x, y)`.
    pub fn blend_pixel(&mut self, x: i32, y: i32, rgba: [u8; 4]) {
        let (w, h) = (self.width() as i32, self.height() as i32);
        if x < 0 || y < 0 || x >= w || y >= h || rgba[3] == 0 {
            return;
        }
        let idx = (y as usize) * (w as usize) + x as usize;
        let dst = self.pixmap.pixels()[idx];
        let px = blend_over(dst, rgba);
        self.pixmap.pixels_mut()[idx] = px;
    }

    /// Source-over blend of a straight-alpha RGBA image at `(x, y)`.
    /// Source-over a straight-alpha RGBA image at `(x, y)`.
    pub fn blend_image(&mut self, x: i32, y: i32, w: u32, h: u32, rgba: &[u8]) {
        if w == 0 || h == 0 || rgba.len() < (w as usize * h as usize * 4) {
            return;
        }
        let cw = self.width() as i32;
        let ch = self.height() as i32;
        let pixels = self.pixmap.pixels_mut();
        for row in 0..h as i32 {
            let dy = y + row;
            if dy < 0 || dy >= ch {
                continue;
            }
            for col in 0..w as i32 {
                let dx = x + col;
                if dx < 0 || dx >= cw {
                    continue;
                }
                let si = ((row as usize) * (w as usize) + col as usize) * 4;
                let a = rgba[si + 3];
                if a == 0 {
                    continue;
                }
                let di = dy as usize * cw as usize + dx as usize;
                pixels[di] = blend_over(pixels[di], [rgba[si], rgba[si + 1], rgba[si + 2], a]);
            }
        }
    }
}

/// Straight-alpha `src` composited over premultiplied `dst`.
#[inline]
fn blend_over(dst: PremultipliedColorU8, src: [u8; 4]) -> PremultipliedColorU8 {
    let a = src[3] as u32;
    if a == 255 {
        return PremultipliedColorU8::from_rgba(src[0], src[1], src[2], 255)
            .unwrap_or_else(|| PremultipliedColorU8::TRANSPARENT);
    }
    let ia = 255 - a;
    let dr = dst.red() as u32;
    let dg = dst.green() as u32;
    let db = dst.blue() as u32;
    let da = dst.alpha() as u32;
    let r = ((src[0] as u32 * a + dr * ia + 127) / 255) as u8;
    let g = ((src[1] as u32 * a + dg * ia + 127) / 255) as u8;
    let b = ((src[2] as u32 * a + db * ia + 127) / 255) as u8;
    let na = ((a * 255 + da * ia + 127) / 255) as u8;
    let r = r.min(na);
    let g = g.min(na);
    let b = b.min(na);
    PremultipliedColorU8::from_rgba(r, g, b, na).unwrap_or(PremultipliedColorU8::TRANSPARENT)
}

/// Box-filter downscale of straight-alpha RGBA data. Used by the small "blob"
/// probe, which looks like a thumbnail of the larger abstract image.
pub fn resize_box(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<u8> {
    let mut out = vec![0u8; dw as usize * dh as usize * 4];
    if sw == 0 || sh == 0 || dw == 0 || dh == 0 {
        return out;
    }
    for y in 0..dh {
        let y0 = (y as u64 * sh as u64 / dh as u64) as u32;
        let y1 = (((y as u64 + 1) * sh as u64 / dh as u64) as u32)
            .max(y0 + 1)
            .min(sh);
        for x in 0..dw {
            let x0 = (x as u64 * sw as u64 / dw as u64) as u32;
            let x1 = (((x as u64 + 1) * sw as u64 / dw as u64) as u32)
                .max(x0 + 1)
                .min(sw);
            let mut acc = [0u64; 4];
            let mut n = 0u64;
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let i = (sy as usize * sw as usize + sx as usize) * 4;
                    for c in 0..4 {
                        acc[c] += src[i + c] as u64;
                    }
                    n += 1;
                }
            }
            let o = (y as usize * dw as usize + x as usize) * 4;
            for c in 0..4 {
                out[o + c] = if n == 0 { 0 } else { (acc[c] / n) as u8 };
            }
        }
    }
    out
}

/// Simple polygon path helper.
pub fn polygon_path(points: &[(f32, f32)]) -> Option<Path> {
    if points.len() < 2 {
        return None;
    }
    let mut pb = PathBuilder::new();
    pb.move_to(points[0].0, points[0].1);
    for &(x, y) in &points[1..] {
        pb.line_to(x, y);
    }
    pb.close();
    pb.finish()
}

/// Circle path helper.
pub fn circle_path(cx: f32, cy: f32, r: f32) -> Option<Path> {
    let mut pb = PathBuilder::new();
    pb.push_circle(cx, cy, r);
    pb.finish()
}
