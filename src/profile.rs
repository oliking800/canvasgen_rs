//! Profile and output types shared by every probe.

use std::path::Path;

use crate::canvas::Canvas;
use crate::emoji::EmojiFont;

/// Which renderer produced a [`GeneratedCanvas`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProbeKind {
    /// 2D text/emoji row (`canvas_3` in the capture set).
    Text,
    /// WebGL-style gradient triangle (`canvas_1`).
    Triangle,
    /// Small 20x20 emoji face (`canvas_0`).
    Small,
    /// Small 20x20 blurred abstract thumbnail (`canvas_2`).
    SmallBlob,
    /// Seeded abstract shapes on an orange field (`canvas_4`).
    Abstract,
}

impl ProbeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ProbeKind::Text => "text",
            ProbeKind::Triangle => "triangle",
            ProbeKind::Small => "small",
            ProbeKind::SmallBlob => "small-blob",
            ProbeKind::Abstract => "abstract",
        }
    }
}

/// Hard ceiling per axis for generated raster canvases (16.7M pixels at
/// 4096x4096). Larger requests are clamped by the infallible API and rejected
/// by [`CanvasProfile::validate`].
pub const MAX_CANVAS_DIM: u32 = 4096;

/// Inputs that control generation.
///
/// `width`, `height` and `dpr` are logical CSS pixels; the raster output is
/// `width * dpr` by `height * dpr`. `font` is raw font bytes (any TTF/OTF/TTC
/// the process can read); when `None`, system emoji discovery is used.
#[derive(Clone, Debug)]
pub struct CanvasProfile {
    pub width: u32,
    pub height: u32,
    pub dpr: f32,
    pub seed: u64,
    pub font: Option<Vec<u8>>,
    pub font_index: u32,
    pub antialias: bool,
    pub background: Option<[u8; 4]>,
}

impl Default for CanvasProfile {
    fn default() -> Self {
        Self {
            width: 300,
            height: 150,
            dpr: 1.0,
            seed: 1,
            font: None,
            font_index: 0,
            antialias: true,
            background: None,
        }
    }
}

impl CanvasProfile {
    /// New profile with `seed` and the defaults (300x150, DPR 1).
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            ..Self::default()
        }
    }

    /// Replace the logical width/height.
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Replace the deterministic seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Use raw font bytes instead of system font discovery.
    pub fn with_font_bytes(mut self, bytes: Vec<u8>) -> Self {
        self.font = Some(bytes);
        self
    }

    /// Read a font file and use it instead of system font discovery.
    pub fn with_font_path(mut self, path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        self.font = Some(bytes);
        Ok(self)
    }

    /// Select a face inside a TrueType collection (default 0).
    pub fn with_font_index(mut self, index: u32) -> Self {
        self.font_index = index;
        self
    }

    /// Output raster size (logical size multiplied by DPR).
    ///
    /// Degenerate input is clamped rather than rejected: zero/oversized
    /// dimensions become 1..=[`MAX_CANVAS_DIM`], non-finite or out-of-range DPR
    /// becomes 1.0/0.25..=8.0. Use [`CanvasProfile::validate`] to get errors.
    pub fn pixel_size(&self) -> (u32, u32) {
        let dpr = if self.dpr.is_finite() {
            self.dpr.clamp(0.25, 8.0)
        } else {
            1.0
        };
        (
            ((self.width as f32 * dpr).round() as u32).clamp(1, MAX_CANVAS_DIM),
            ((self.height as f32 * dpr).round() as u32).clamp(1, MAX_CANVAS_DIM),
        )
    }

    /// Reject dimensions, DPR or font bytes that the renderer should not
    /// silently clamp. Used by the `try_*` entry points.
    pub fn validate(&self) -> Result<(), String> {
        if self.width == 0 || self.height == 0 {
            return Err("canvas width and height must be >= 1".into());
        }
        if !self.dpr.is_finite() || self.dpr <= 0.0 {
            return Err("dpr must be finite and > 0".into());
        }
        let raw_w = (self.width as f32 * self.dpr).round();
        let raw_h = (self.height as f32 * self.dpr).round();
        if raw_w > MAX_CANVAS_DIM as f32 || raw_h > MAX_CANVAS_DIM as f32 {
            return Err(format!(
                "raster size {raw_w}x{raw_h} exceeds {MAX_CANVAS_DIM}px per axis"
            ));
        }
        if let Some(bytes) = &self.font {
            EmojiFont::from_bytes(bytes.clone(), self.font_index, "embedded")
                .map_err(|e| format!("font: {e}"))?;
        }
        Ok(())
    }

    pub(crate) fn emoji_font(&self) -> Result<EmojiFont, String> {
        match &self.font {
            Some(bytes) => EmojiFont::from_bytes(bytes.clone(), self.font_index, "embedded"),
            None => crate::emoji::system_emoji_font(),
        }
    }
}

/// A rendered probe.
#[derive(Clone, Debug)]
pub struct GeneratedCanvas {
    pub kind: ProbeKind,
    pub width: u32,
    pub height: u32,
    /// Straight-alpha RGBA, top-left origin.
    pub rgba: Vec<u8>,
    /// Encoded PNG bytes.
    pub png: Vec<u8>,
    /// `data:image/png;base64,...`
    pub data_url: String,
}

impl GeneratedCanvas {
    /// Encode an in-memory canvas.
    pub fn from_canvas(kind: ProbeKind, canvas: &Canvas) -> Self {
        Self::from_rgba(kind, canvas.width(), canvas.height(), canvas.to_rgba())
    }

    /// Encode RGBA bytes.
    ///
    /// # Panics
    /// Panics only if the PNG encoder rejects an internal buffer (never for
    /// canvases produced by this crate). Use [`GeneratedCanvas::try_from_rgba`]
    /// for untrusted data.
    pub fn from_rgba(kind: ProbeKind, width: u32, height: u32, rgba: Vec<u8>) -> Self {
        Self::try_from_rgba(kind, width, height, rgba).expect("png encode")
    }

    /// Fallible RGBA encoder for caller-supplied buffers.
    pub fn try_from_rgba(
        kind: ProbeKind,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    ) -> Result<Self, String> {
        if width == 0 || height == 0 {
            return Err("width and height must be >= 1".into());
        }
        let expected = width as usize * height as usize * 4;
        if rgba.len() != expected {
            return Err(format!(
                "rgba buffer has {} bytes, expected {expected}",
                rgba.len()
            ));
        }
        let png = crate::pngout::encode_rgba(width, height, &rgba)?;
        let data_url = crate::pngout::to_data_url(&png);
        Ok(Self {
            kind,
            width,
            height,
            rgba,
            png,
            data_url,
        })
    }

    /// The PNG as a browser-style data URL.
    pub fn to_data_url(&self) -> String {
        self.data_url.clone()
    }

    /// `img.src` value for this probe.
    pub fn url(&self) -> &str {
        &self.data_url
    }
}
