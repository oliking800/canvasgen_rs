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
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            ..Self::default()
        }
    }

    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    pub fn with_font_bytes(mut self, bytes: Vec<u8>) -> Self {
        self.font = Some(bytes);
        self
    }

    pub fn with_font_path(mut self, path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        self.font = Some(bytes);
        Ok(self)
    }

    pub fn with_font_index(mut self, index: u32) -> Self {
        self.font_index = index;
        self
    }

    /// Output raster size (logical size multiplied by DPR, clamped to at least
    /// one pixel per axis).
    pub fn pixel_size(&self) -> (u32, u32) {
        let dpr = if self.dpr.is_finite() {
            self.dpr.clamp(0.25, 8.0)
        } else {
            1.0
        };
        (
            ((self.width as f32 * dpr).round() as u32).max(1),
            ((self.height as f32 * dpr).round() as u32).max(1),
        )
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
    pub fn from_canvas(kind: ProbeKind, canvas: &Canvas) -> Self {
        Self::from_rgba(kind, canvas.width(), canvas.height(), canvas.to_rgba())
    }

    pub fn from_rgba(kind: ProbeKind, width: u32, height: u32, rgba: Vec<u8>) -> Self {
        let png = crate::pngout::encode_rgba(width, height, &rgba).expect("png encode");
        let data_url = crate::pngout::to_data_url(&png);
        Self {
            kind,
            width,
            height,
            rgba,
            png,
            data_url,
        }
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
