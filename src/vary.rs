//! Per-device micro-variation of captured canvas bases.
//!
//! `class`-mode bases carry the correct per-platform artwork (Apple/Noto
//! emoji, engine rasterization) but are a single capture. Two devices of the
//! same class still differ slightly. This module applies the measured
//! envelopes to a base and re-encodes it:
//!
//! | probe | measured same-class variation |
//! |---|---|
//! | WebGL triangle | 0â€“3.6 % of pixels, **every delta Â±1 LSB**, contour-shaped |
//! | text row (300Ã—150) | 0â€“1.1 % of pixels, mean â‰¤0.25 |
//! | face (20Ã—20) | 0â€“65 % of pixels, mean â‰¤10.8 |
//! | abstract probes | 8â€“90 % of pixels, mean 0.1â€“12 |
//!
//! The variation is deterministic in [`VaryProfile`], so the same profile
//! always yields byte-identical output and different profiles get different
//! devices.

use std::io::Cursor;

use crate::pngout;
use crate::profile::ProbeKind;

/// Measured-bounded per-device variation parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VaryProfile {
    /// Sub-LSB rounding bias per channel (Â±0.25 LSB); only affects the WebGL
    /// gradient's quantization contours.
    pub rgb_bias: [f32; 3],
    /// Edge-weighted text raster delta (fraction of 255).
    pub text_delta: f32,
    /// Compositor gain for dense-gradient probes (1 Â± 0.004).
    pub comp_scale: f32,
    /// Compositor offset for dense-gradient probes (Â±1 LSB).
    pub comp_bias: f32,
    /// Pattern salt for the text delta.
    pub salt: u32,
}

impl Default for VaryProfile {
    fn default() -> Self {
        Self::neutral()
    }
}

impl VaryProfile {
    /// No variation (returns the base pixels, re-encoded).
    pub const fn neutral() -> Self {
        Self {
            rgb_bias: [0.0, 0.0, 0.0],
            text_delta: 0.0,
            comp_scale: 1.0,
            comp_bias: 0.0,
            salt: 0,
        }
    }

    /// Stable per-profile parameters drawn from the measured same-class
    /// envelopes.
    pub fn from_seed(seed: u64) -> Self {
        fn mix(mut x: u64) -> u64 {
            x ^= x >> 30;
            x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
            x ^= x >> 27;
            x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
            x ^ (x >> 31)
        }
        let h = mix(seed ^ 0xB453_1A5E_0000_0001);
        let unit = |salt: u64| -> f32 { ((mix(h ^ salt) >> 11) as f32) / ((1u64 << 53) as f32) };
        let snap = |v: f32, step: f32| (v / step).round() * step;
        let signed = |salt: u64| unit(salt) * 2.0 - 1.0;
        Self {
            rgb_bias: [
                snap(signed(1) * 0.25, 1.0 / 64.0),
                snap(signed(2) * 0.25, 1.0 / 64.0),
                snap(signed(3) * 0.25, 1.0 / 64.0),
            ],
            text_delta: snap(0.02 + unit(4) * 0.10, 1.0 / 512.0),
            comp_scale: snap(1.0 + signed(5) * 0.004, 1.0 / 4096.0),
            comp_bias: snap(signed(6), 1.0 / 64.0),
            salt: (mix(h ^ 0x5A17_C0DE) >> 32) as u32,
        }
    }
}

/// Apply `vary` to a PNG data URL and return a new data URL.
pub fn vary_canvas_data_url(
    url: &str,
    kind: ProbeKind,
    vary: &VaryProfile,
) -> Result<String, String> {
    let (w, h, mut rgba) = decode(url)?;
    match kind {
        ProbeKind::Triangle => vary_triangle(&mut rgba, w, h, vary),
        ProbeKind::Text | ProbeKind::Small => {
            crate::probes::perturb_text_edges(&mut rgba, w, h, vary.text_delta, vary.salt)
        }
        ProbeKind::Abstract | ProbeKind::SmallBlob => {
            vary_compositor(&mut rgba, vary);
        }
    }
    let png = pngout::encode_rgba(w, h, &rgba)?;
    Ok(pngout::to_data_url(&png))
}

fn decode(url: &str) -> Result<(u32, u32, Vec<u8>), String> {
    let bytes = pngout::decode_data_url(url)?;
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(
        png::Transformations::EXPAND | png::Transformations::ALPHA | png::Transformations::STRIP_16,
    );
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("png header: {e}"))?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("png decode: {e}"))?;
    let (w, h) = (info.width, info.height);
    let src = &buf[..info.buffer_size()];
    let mut rgba = Vec::with_capacity(w as usize * h as usize * 4);
    match info.color_type {
        png::ColorType::Rgba => rgba.extend_from_slice(src),
        png::ColorType::Rgb => {
            for px in src.chunks_exact(3) {
                rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for px in src.chunks_exact(2) {
                rgba.extend_from_slice(&[px[0], px[0], px[0], px[1]]);
            }
        }
        png::ColorType::Grayscale => {
            for &v in src {
                rgba.extend_from_slice(&[v, v, v, 255]);
            }
        }
        other => return Err(format!("png: unsupported colour type {other:?}")),
    }
    if rgba.len() != w as usize * h as usize * 4 {
        return Err("png: decoded size mismatch".into());
    }
    Ok((w, h, rgba))
}

/// Contour-shaped Â±1-LSB flips: re-quantize the known gradient with a new
/// sub-LSB bias, but only for interior pixels that still match the ideal.
fn vary_triangle(rgba: &mut [u8], w: u32, h: u32, vary: &VaryProfile) {
    if vary.rgb_bias == [0.0, 0.0, 0.0] {
        return;
    }
    let wf = w as f32;
    let hf = h as f32;
    for y in 0..h {
        for x in 0..w {
            let i = (y as usize * w as usize + x as usize) * 4;
            let px = &mut rgba[i..i + 4];
            if px[3] != 255 {
                continue;
            }
            let (r, g, b) = (px[0] as f32, px[1] as f32, px[2] as f32);
            if r + g + b < 12.0 {
                continue; // black field
            }
            let ideal_r = (2.0 * (x as f32 + 0.5) / wf).clamp(0.0, 1.0) * 255.0;
            let ideal_g = (2.0 * (hf - (y as f32 + 0.5)) / hf).clamp(0.0, 1.0) * 255.0;
            // Only touch pixels that are the un-antialiased gradient.
            if (r - ideal_r).abs() > 1.0 || (g - ideal_g).abs() > 1.0 || (b - 255.0).abs() > 1.0 {
                continue;
            }
            let nr = (ideal_r + vary.rgb_bias[0]).round().clamp(0.0, 255.0) as i32;
            let ng = (ideal_g + vary.rgb_bias[1]).round().clamp(0.0, 255.0) as i32;
            let nb = (255.0 + vary.rgb_bias[2]).round().clamp(0.0, 255.0) as i32;
            // Keep it micro: never move a channel by more than one LSB.
            if (nr - px[0] as i32).abs() <= 1 {
                px[0] = nr as u8;
            }
            if (ng - px[1] as i32).abs() <= 1 {
                px[1] = ng as u8;
            }
            if (nb - px[2] as i32).abs() <= 1 {
                px[2] = nb as u8;
            }
        }
    }
}

/// Small per-channel gain/offset on non-transparent pixels, standing in for
/// compositor/AA rounding differences in the dense-gradient probes.
fn vary_compositor(rgba: &mut [u8], vary: &VaryProfile) {
    if vary.comp_scale == 1.0 && vary.comp_bias == 0.0 {
        return;
    }
    for px in rgba.chunks_exact_mut(4) {
        if px[3] == 0 {
            continue;
        }
        for c in 0..3 {
            let v = px[c] as f32 * vary.comp_scale + vary.comp_bias;
            px[c] = v.round().clamp(0.0, 255.0) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_is_stable_and_bounded() {
        let a = VaryProfile::from_seed(7);
        assert_eq!(a, VaryProfile::from_seed(7));
        assert_ne!(a, VaryProfile::from_seed(8));
        assert!(a.rgb_bias.iter().all(|b| b.abs() <= 0.25));
        assert!((0.02..=0.12).contains(&a.text_delta));
        assert!((0.996..=1.004).contains(&a.comp_scale));
        assert!(a.comp_bias.abs() <= 1.0);
    }
}
