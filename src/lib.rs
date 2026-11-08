//! # canvasgen
//!
//! Deterministic, plausible browser canvas fingerprints, rendered in pure Rust.
//!
//! The crate reproduces the five canvas probes hCaptcha asks a browser for
//! (`canvasDataURLs`), plus full-colour emoji text rendering:
//!
//! ```no_run
//! use canvasgen::{fingerprint_set, CanvasProfile};
//!
//! let profile = CanvasProfile::new(42);
//! let canvases = fingerprint_set(&profile);
//! assert_eq!(canvases.len(), 5);
//! println!("{}", canvases[3].to_data_url());
//! ```
//!
//! * [`emoji_text`] / [`emoji_row`] — 2D emoji row (COLRv1, COLRv0, CBDT, sbix)
//! * [`gradient_triangle`] — WebGL-style RGB triangle
//! * [`small_probe`] — 20x20 emoji face or blurred abstract thumbnail
//! * [`abstract_image`] — seeded shapes + text on orange
//! * [`fingerprint_set`] — the five probes in capture order
//!
//! Same seed + profile (and same platform/build) produce byte-identical PNGs.

pub mod canvas;
pub mod emoji;
pub mod pngout;
pub mod probes;
pub mod profile;

pub use canvas::Canvas;
pub use emoji::{
    measure, render_glyph, render_row, shape, system_emoji_font, system_font_info, ColorSupport,
    EmojiFont, EmojiProfile, RowStats, ShapedGlyph, ShapedRun,
};
pub use probes::{
    abstract_image, canvas_data_urls, emoji_row, emoji_text, emoji_text_fallback, fingerprint_set,
    gradient_triangle, render_probe, small_probe, small_probe_with, SmallKind, CANVAS_CTXS_JSON,
    DEFAULT_EMOJI_ROW, FINGERPRINT_PROBES,
};
pub use profile::{CanvasProfile, GeneratedCanvas, ProbeKind, MAX_CANVAS_DIM};

/// Fallible [`render_probe`]: rejects zero/oversized dimensions, invalid DPR
/// and invalid font bytes instead of clamping/falling back.
pub fn try_render_probe(
    kind: ProbeKind,
    profile: &CanvasProfile,
) -> Result<GeneratedCanvas, String> {
    profile.validate()?;
    Ok(render_probe(kind, profile))
}

/// Fallible [`fingerprint_set`].
pub fn try_fingerprint_set(profile: &CanvasProfile) -> Result<Vec<GeneratedCanvas>, String> {
    profile.validate()?;
    Ok(fingerprint_set(profile))
}

/// Fallible [`canvas_data_urls`].
pub fn try_canvas_data_urls(profile: &CanvasProfile) -> Result<Vec<String>, String> {
    Ok(try_fingerprint_set(profile)?
        .into_iter()
        .map(|c| c.data_url)
        .collect())
}

/// Package version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
