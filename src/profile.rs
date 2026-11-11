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

/// Per-device rasterizer micro-parameters.
///
/// Real GPU output differs by sub-LSB rounding of interpolated colour.
/// Measured on the same hCaptcha WebGL triangle: an Apple GPU and an NVIDIA
/// GPU differ on 3.89% of pixels and **every difference is exactly 1 LSB**,
/// forming quantization contours; two browsers on the same GPU differ by 0.
/// Repeated visits on one device also differ by 0.
///
/// Variation is therefore modelled per **GPU family** ([`GpuClass`]), not per
/// unit: `rgb_bias` is the family's bias in 8-bit LSB units, applied to the
/// interpolated colour before quantization. Same family -> identical output;
/// different family -> 1-LSB contour flips on a few percent of pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SiliconProfile {
    pub rgb_bias: [f32; 3],
}

impl Default for SiliconProfile {
    fn default() -> Self {
        Self::neutral()
    }
}

/// GPU/driver family. Interpolation rounding is a property of the family
/// (and driver), not of an individual unit: measured iPhone-to-iPhone pairs
/// are either bit-identical or differ only by 1-LSB contours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuClass {
    Nvidia,
    /// NVIDIA Ada/Blackwell (RTX 40/50) under ANGLE/D3D11: measured on an RTX 4060,
    /// where the green ramp rounds about -0.1 LSB lower than the older reference.
    NvidiaAda,
    Amd,
    Intel,
    Apple,
    Adreno,
    Mali,
    Unknown,
}

impl GpuClass {
    /// Sub-LSB interpolation bias per channel (8-bit LSB units), calibrated
    /// against the measured pairs:
    /// * NVIDIA (reference) 0.00,
    /// * Apple about -0.20 on green (3.89 % of triangle pixels flip),
    /// * AMD and Adreno are inferred from 1.7â€“4.4 % flips, max 1â€“2 LSB.
    pub const fn bias(self) -> [f32; 3] {
        match self {
            GpuClass::Nvidia => [0.0, 0.0, 0.0],
            GpuClass::NvidiaAda => [0.0, -0.10, 0.0],
            GpuClass::Amd => [0.0, -0.10, 0.0],
            GpuClass::Intel => [0.0, 0.05, 0.0],
            GpuClass::Apple => [0.0, -0.20, 0.0],
            GpuClass::Adreno => [0.0, 0.25, 0.0],
            GpuClass::Mali => [0.0, 0.15, 0.0],
            GpuClass::Unknown => [0.0, 0.0, 0.0],
        }
    }
}

impl GpuClass {
    /// GPU family from a WebGL `UNMASKED_RENDERER_WEBGL` string (for example
    /// `ANGLE (NVIDIA, NVIDIA GeForce RTX 4060 (0x00002882) Direct3D11 ...)`),
    /// so the triangle probe agrees with the GPU the fingerprint claims.
    pub fn from_renderer(renderer: &str) -> GpuClass {
        let r = renderer.to_ascii_lowercase();
        if r.contains("nvidia") || r.contains("geforce") || r.contains("quadro") {
            // RTX 40xx / 50xx: the model number is the four digits after "rtx".
            let ada = r.find("rtx").is_some_and(|i| {
                let digits: String = r[i + 3..]
                    .chars()
                    .skip_while(|c| !c.is_ascii_digit())
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                digits.len() >= 4 && matches!(digits.as_bytes()[0], b'4' | b'5')
            });
            if ada { GpuClass::NvidiaAda } else { GpuClass::Nvidia }
        } else if r.contains("radeon") || r.contains("amd") {
            GpuClass::Amd
        } else if r.contains("apple") {
            GpuClass::Apple
        } else if r.contains("adreno") {
            GpuClass::Adreno
        } else if r.contains("mali") {
            GpuClass::Mali
        } else if r.contains("intel") {
            GpuClass::Intel
        } else {
            GpuClass::Unknown
        }
    }
}

impl SiliconProfile {
    /// Bias-free rasterizer (the exact captured device).
    pub const fn neutral() -> Self {
        Self {
            rgb_bias: [0.0, 0.0, 0.0],
        }
    }
    /// Bias for the GPU named by a WebGL renderer string.
    pub fn for_renderer(renderer: &str) -> Self {
        Self::for_gpu(GpuClass::from_renderer(renderer))
    }
    /// Bias for a known GPU family.
    pub const fn for_gpu(gpu: GpuClass) -> Self {
        Self {
            rgb_bias: gpu.bias(),
        }
    }

    /// Pick a GPU class deterministically from a device seed (weighted roughly
    /// by desktop/mobile market share), then return its fixed bias.
    pub fn from_seed(seed: u64) -> Self {
        fn mix(mut x: u64) -> u64 {
            x ^= x >> 30;
            x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
            x ^= x >> 27;
            x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
            x ^ (x >> 31)
        }
        const CLASSES: [GpuClass; 12] = [
            GpuClass::Nvidia,
            GpuClass::Nvidia,
            GpuClass::Nvidia,
            GpuClass::Amd,
            GpuClass::Amd,
            GpuClass::Intel,
            GpuClass::Intel,
            GpuClass::Apple,
            GpuClass::Apple,
            GpuClass::Adreno,
            GpuClass::Mali,
            GpuClass::Unknown,
        ];
        Self::for_gpu(CLASSES[(mix(seed) % CLASSES.len() as u64) as usize])
    }
}

/// Text rasterizer family. The measured Chromeâ†”Firefox emoji row differs on
/// 0.84 % of the canvas (mean |Î”| 0.28) and the 20Ã—20 face on 38.5 % (mean
/// 4.4) with the same fonts and GPU â€” an origin/AA difference, not artwork.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextEngine {
    /// Chrome/Skia (reference).
    Skia,
    /// Firefox/Gecko.
    Gecko,
    /// Safari/WebKit.
    WebKit,
}

impl TextEngine {
    /// Measured origin offset and alpha gamma relative to Skia.
    pub const fn style(self) -> (f32, f32, f32) {
        match self {
            TextEngine::Skia => (0.0, 0.0, 1.0),
            TextEngine::Gecko => (0.30, 0.06, 1.01),
            TextEngine::WebKit => (-0.22, 0.08, 0.99),
        }
    }

    /// Browser engine implied by a user agent string (all iOS browsers are
    /// WebKit; Firefox/Gecko only on desktop/Android).
    pub fn from_user_agent(ua: &str) -> Self {
        let ua = ua.to_ascii_lowercase();
        if ua.contains("iphone") || ua.contains("ipad") || ua.contains("ipod") {
            TextEngine::WebKit
        } else if ua.contains("firefox") || ua.contains("fxios") {
            TextEngine::Gecko
        } else if ua.contains("safari")
            && !ua.contains("chrome")
            && !ua.contains("chromium")
            && !ua.contains("edg")
            && !ua.contains("opr")
        {
            TextEngine::WebKit
        } else {
            TextEngine::Skia
        }
    }
}

/// Per-device text micro-parameters: subpixel origin offset, alpha gamma and
/// the engine that sets the baseline offset. `dx`/`dy` are logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    pub engine: TextEngine,
    pub dx: f32,
    pub dy: f32,
    pub gamma: f32,
    /// Engine-level colour-glyph rasterization delta (fraction of 255),
    /// applied to glyph edges. 0 for the reference engine.
    pub raster_delta: f32,
    /// Pattern salt so different devices of the same engine differ, while the
    /// same device stays identical.
    pub salt: u32,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self::neutral()
    }
}

impl TextStyle {
    /// Exact captured rasterizer (no shift, gamma 1, no raster delta).
    pub const fn neutral() -> Self {
        Self {
            engine: TextEngine::Skia,
            dx: 0.0,
            dy: 0.0,
            gamma: 1.0,
            raster_delta: 0.0,
            salt: 0x51ED_0001,
        }
    }

    /// Fixed style for a text engine family.
    pub const fn for_engine(engine: TextEngine) -> Self {
        let (dx, dy, gamma) = engine.style();
        let raster_delta = match engine {
            TextEngine::Skia => 0.0,
            TextEngine::Gecko => 0.17,
            TextEngine::WebKit => 0.15,
        };
        Self {
            engine,
            dx,
            dy,
            gamma,
            raster_delta,
            salt: 0x51ED_0001,
        }
    }

    /// Style matching the browser engine in a user agent string.
    pub fn for_user_agent(ua: &str) -> Self {
        Self::for_engine(TextEngine::from_user_agent(ua))
    }

    /// Stable per-device style. Text rasterization is class-level in the
    /// measurements (two Chrome installs are identical; two Firefox installs
    /// differ only when font smoothing/DPI/config differs), so the seed picks
    /// an engine class and returns its fixed style. Use the public fields to
    /// model a non-default DPI/font configuration explicitly.
    pub fn from_seed(seed: u64) -> Self {
        fn mix(mut x: u64) -> u64 {
            x ^= x >> 30;
            x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
            x ^= x >> 27;
            x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
            x ^ (x >> 31)
        }
        let engine = match mix(seed ^ 0x7E57_0000_0000_0001) % 20 {
            0..=5 => TextEngine::Gecko,
            6..=8 => TextEngine::WebKit,
            _ => TextEngine::Skia,
        };
        Self::for_engine(engine)
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
