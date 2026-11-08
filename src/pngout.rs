//! Deterministic PNG + data-URL encoding.

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use std::io::Cursor;

/// Encode straight-alpha RGBA data as an 8-bit RGBA PNG.
///
/// The encoder settings are fixed (no filtering, default compression) so the
/// same pixels always produce byte-identical output on a given `png` version.
pub fn encode_rgba(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let expected = width as usize * height as usize * 4;
    if rgba.len() < expected {
        return Err(format!(
            "rgba buffer too short: {} < {expected}",
            rgba.len()
        ));
    }
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(Cursor::new(&mut out), width, height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::Default);
        enc.set_filter(png::FilterType::Paeth);
        let mut writer = enc.write_header().map_err(|e| format!("png header: {e}"))?;
        writer
            .write_image_data(&rgba[..expected])
            .map_err(|e| format!("png data: {e}"))?;
        writer.finish().map_err(|e| format!("png finish: {e}"))?;
    }
    Ok(out)
}

pub fn to_data_url(png: &[u8]) -> String {
    format!("data:image/png;base64,{}", STANDARD.encode(png))
}

pub fn decode_data_url(url: &str) -> Result<Vec<u8>, String> {
    let rest = url
        .strip_prefix("data:image/png;base64,")
        .ok_or_else(|| "not a PNG data URL".to_string())?;
    STANDARD.decode(rest).map_err(|e| format!("base64: {e}"))
}

/// Minimal JSON string escaping for the CLI output (`--font` paths never show
/// up in these strings, but base64 can contain `+` and `/` only).
pub fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
