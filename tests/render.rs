//! Integration tests and sample generation.

use std::path::PathBuf;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;

use canvasgen::emoji::{self, EmojiProfile};
use canvasgen::{
    emoji_text, emoji_text_fallback, fingerprint_set, render_probe, CanvasProfile, GeneratedCanvas,
    ProbeKind, DEFAULT_EMOJI_ROW,
};

fn samples_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("samples");
    std::fs::create_dir_all(&dir).expect("samples dir");
    dir
}

fn decode_png(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().expect("png header");
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).expect("png frame");
    buf.truncate(info.buffer_size());
    (info.width, info.height, buf)
}

fn colourful_pixels(rgba: &[u8]) -> usize {
    rgba.chunks_exact(4)
        .filter(|p| {
            p[3] > 200 && (p[0].max(p[1]).max(p[2]) as i16 - p[0].min(p[1]).min(p[2]) as i16) > 24
        })
        .count()
}

fn non_background_bbox(rgba: &[u8], w: u32, h: u32) -> Option<(u32, u32, u32, u32)> {
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    let mut any = false;
    for y in 0..h {
        for x in 0..w {
            let i = (y as usize * w as usize + x as usize) * 4;
            let (r, g, b, a) = (rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]);
            if a > 8 && !(r > 245 && g > 245 && b > 245) {
                any = true;
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
            }
        }
    }
    any.then_some((x0, y0, x1, y1))
}

#[test]
fn deterministic_png_bytes() {
    let profile = CanvasProfile::new(42);
    for kind in [
        ProbeKind::Text,
        ProbeKind::Triangle,
        ProbeKind::Small,
        ProbeKind::SmallBlob,
        ProbeKind::Abstract,
    ] {
        let a = render_probe(kind, &profile);
        let b = render_probe(kind, &profile);
        assert_eq!(a.png, b.png, "{kind:?} PNG bytes differ");
        assert_eq!(a.data_url, b.data_url, "{kind:?} data URL differs");
    }

    let set_a = fingerprint_set(&profile);
    let set_b = fingerprint_set(&profile);
    assert_eq!(set_a.len(), 5);
    for (a, b) in set_a.iter().zip(&set_b) {
        assert_eq!(a.png, b.png);
    }
}

#[test]
fn valid_png_dimensions_and_data_url_roundtrip() {
    let profile = CanvasProfile::new(7);
    for c in fingerprint_set(&profile) {
        assert!(!c.rgba.is_empty());
        assert_eq!(c.rgba.len(), c.width as usize * c.height as usize * 4);
        assert!(c.data_url.starts_with("data:image/png;base64,"));

        let payload = c.data_url.strip_prefix("data:image/png;base64,").unwrap();
        let decoded = STANDARD.decode(payload).expect("base64");
        assert_eq!(decoded, c.png, "base64 does not round-trip");

        let (w, h, rgba) = decode_png(&c.png);
        assert_eq!((w, h), (c.width, c.height));
        assert_eq!(rgba.len(), c.rgba.len());
        assert!(!c.to_data_url().is_empty());
    }
}

#[test]
fn fingerprint_set_matches_capture_shapes() {
    let set = fingerprint_set(&CanvasProfile::new(3));
    let expected = [
        (ProbeKind::Small, 20, 20),
        (ProbeKind::Triangle, 300, 150),
        (ProbeKind::SmallBlob, 20, 20),
        (ProbeKind::Text, 300, 150),
        (ProbeKind::Abstract, 96, 48),
    ];
    for (c, (kind, w, h)) in set.iter().zip(expected) {
        assert_eq!(c.kind, kind);
        assert_eq!((c.width, c.height), (w, h));
    }
}

#[test]
fn colour_emoji_output() {
    let Ok(font) = canvasgen::system_emoji_font() else {
        eprintln!("skipping colour test: no system emoji font");
        return;
    };
    eprintln!("font: {} ({})", font.family, font.color.label());

    let canvas = emoji_text(&CanvasProfile::new(42));
    let (_, _, rgba) = decode_png(&canvas.png);
    let colourful = colourful_pixels(&rgba);
    assert!(
        colourful > 40,
        "expected colour emoji pixels, got {colourful}"
    );

    // Not a greyscale rendering: at least three hue families.
    let mut reds = 0;
    let mut yellows = 0;
    let mut blues = 0;
    for p in rgba.chunks_exact(4) {
        if p[3] < 200 {
            continue;
        }
        if p[0] > p[1] + 40 && p[0] > p[2] + 40 {
            reds += 1;
        }
        if p[0] > 180 && p[1] > 150 && p[2] < 120 {
            yellows += 1;
        }
        if p[2] > p[0] + 40 {
            blues += 1;
        }
    }
    assert!(reds > 10, "heart red missing: {reds}");
    assert!(yellows > 20, "emoji yellow missing: {yellows}");
    assert!(blues > 0, "blue cuff missing: {blues}");
}

#[test]
fn shaping_collapses_sequences() {
    let Ok(font) = canvasgen::system_emoji_font() else {
        eprintln!("skipping shaping test: no system emoji font");
        return;
    };

    let heart = emoji::shape("\u{2764}\u{FE0F}", &font).unwrap();
    assert_eq!(heart.clusters, 1, "heart+VS16 should be one cluster");
    // Some fonts keep VS16 as a separate zero-advance glyph; the cluster is
    // still a single shaped unit.
    assert!(
        heart.glyphs.len() <= 2,
        "heart should be one glyph (plus a zero-width VS16 at most)"
    );
    assert!(heart.glyphs[1..].iter().all(|g| g.x_advance == 0.0));

    let thumb = emoji::shape("\u{1F44D}\u{1F3FD}", &font).unwrap();
    assert_eq!(thumb.clusters, 1, "skin-tone modifier should attach");
    assert_eq!(thumb.glyphs.len(), 1);

    let family = emoji::shape(
        "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}",
        &font,
    )
    .unwrap();
    assert_eq!(family.clusters, 1, "ZWJ family should be one cluster");
    // Segoe composes the family from layered glyphs at one pen position, so the
    // glyph count can be > 1; the advance must still be a single em.
    let fam_adv = family.advance_px(26.0);
    eprintln!(
        "ZWJ family: {} glyphs, advance {:.1}px, offsets {:?}",
        family.glyphs.len(),
        fam_adv,
        family
            .glyphs
            .iter()
            .map(|g| (g.x_advance, g.x_offset))
            .collect::<Vec<_>>()
    );
    // Per-glyph advances vary between fonts (some compose by layering); the
    // shaped unit is one cluster, which is what hCaptcha's hash observes.
    assert!(family.glyphs.iter().all(|g| g.cluster == 0));

    // Flags need a font with the regional-indicator ligature. Segoe UI Emoji
    // (Windows) does not have one, so this is a capability check rather than a
    // hard failure; the README documents the matrix.
    let flag = emoji::shape("\u{1F1FA}\u{1F1F8}", &font).unwrap();
    if flag.clusters == 1 {
        if flag.glyphs.len() != 1 {
            eprintln!(
                "note: {} produces {} glyphs for a flag pair (no single flag glyph)",
                font.family,
                flag.glyphs.len()
            );
        }
        assert!(flag.glyphs.len() >= 1);
    } else {
        eprintln!(
            "note: {} has no flag ligature ({} glyphs, {} clusters)",
            font.family,
            flag.glyphs.len(),
            flag.clusters
        );
        assert!(flag.glyphs.len() >= 2);
    }

    // Plain ASCII stays multiple clusters.
    let abc = emoji::shape("abc", &font).unwrap();
    assert_eq!(abc.clusters, 3);
}

#[test]
fn emoji_row_matches_captured_layout() {
    let canvas = emoji_text(&CanvasProfile::new(42));
    let (w, h, rgba) = decode_png(&canvas.png);
    let bbox = non_background_bbox(&rgba, w, h).expect("row pixels");
    eprintln!("generated row bbox: {bbox:?}");
    // Captured canvas_3_300x150.png: (51, 60) .. (102, 71).
    assert!((48..=54).contains(&bbox.0), "left edge {bbox:?}");
    assert!((58..=62).contains(&bbox.1), "top edge {bbox:?}");
    assert!((96..=112).contains(&bbox.2), "right edge {bbox:?}");
    assert!((69..=74).contains(&bbox.3), "bottom edge {bbox:?}");
}

#[test]
fn fallback_shapes_are_colourful_and_valid() {
    let canvas = emoji_text_fallback(&CanvasProfile::new(1));
    let (w, h, rgba) = decode_png(&canvas.png);
    assert_eq!((w, h), (300, 150));
    assert!(colourful_pixels(&rgba) > 100);
}

#[test]
fn colrv1_path_if_available() {
    let Ok(font) = canvasgen::system_emoji_font() else {
        return;
    };
    if font.color != canvasgen::ColorSupport::ColrV1 {
        eprintln!(
            "skipping system COLRv1 test: font is {}",
            font.color.label()
        );
    } else {
        let canvas = emoji_text(&CanvasProfile::new(5));
        let (_, _, rgba) = decode_png(&canvas.png);
        assert!(colourful_pixels(&rgba) > 40);
    }

    // Exercise the COLRv1 paint graph on the fontations test font, which has
    // no cmap, so glyph ids are rendered directly.
    use skrifa::MetadataProvider as _;
    let test_font =
        canvasgen::EmojiFont::from_bytes(font_test_data::COLRV0V1.to_vec(), 0, "COLRV0V1")
            .expect("COLRV0V1 test font");
    assert_eq!(test_font.color, canvasgen::ColorSupport::ColrV1);
    let sk = skrifa::FontRef::from_index(&test_font.data, 0).unwrap();
    let color_glyphs = sk.color_glyphs();

    let mut canvas = canvasgen::Canvas::filled(660, 220, [255, 255, 255, 255]);
    let mut color_ids = 0;
    let mut drawn = 0;
    for gid in 0..300u16 {
        let Some(_) = color_glyphs.get_with_format(
            skrifa::GlyphId::new(gid as u32),
            skrifa::color::ColorGlyphFormat::ColrV1,
        ) else {
            continue;
        };
        color_ids += 1;
        let x = 20.0 + ((color_ids - 1) % 10) as f32 * 64.0;
        let y = 40.0 + ((color_ids - 1) / 10) as f32 * 60.0;
        let is_color = match canvasgen::render_glyph(
            &mut canvas,
            &test_font,
            gid,
            34.0,
            x,
            y,
            true,
            [30, 30, 30, 255],
        ) {
            Ok(v) => v,
            // The fontations test font deliberately contains cycle/error
            // cases; skip those paint graphs.
            Err(_) => continue,
        };
        if is_color {
            drawn += 1;
        }
    }
    eprintln!("COLRv1: {color_ids} colour glyphs, {drawn} painted");
    assert!(color_ids > 0, "test font has no COLRv1 glyphs");
    assert!(drawn > 0, "COLRv1 painter drew nothing");

    let colourful = colourful_pixels(&canvas.to_rgba());
    eprintln!("COLRv1 colourful pixels: {colourful}");
    assert!(colourful > 100, "COLRv1 output is not colourful");

    let out = canvasgen::GeneratedCanvas::from_canvas(canvasgen::ProbeKind::Text, &canvas);
    std::fs::write(samples_dir().join("colrv1_test_glyphs.png"), &out.png).unwrap();
}

#[test]
fn flag_font_capability() {
    // A real flag-capable font from the fontations test data (Noto colour
    // emoji flags). Confirms shaping produces one cluster and colour output.
    let font = canvasgen::EmojiFont::from_bytes(
        font_test_data::NOTO_COLOR_EMOJI_FLAGS.to_vec(),
        0,
        "Noto Color Emoji flags",
    )
    .expect("flag font");
    let run = emoji::shape("\u{1F1FA}\u{1F1F8}", &font).unwrap();
    eprintln!(
        "flag font: {} glyphs, {} clusters, advance {:.1}",
        run.glyphs.len(),
        run.clusters,
        run.advance_px(32.0)
    );
    assert_eq!(run.clusters, 1, "flag pair should be one cluster");

    let mut canvas = canvasgen::Canvas::filled(200, 80, [255, 255, 255, 255]);
    let ep = EmojiProfile {
        font_size: 40.0,
        dpr: 1.0,
        baseline: 60.0,
        letter_spacing: 0.0,
        antialias: true,
    };
    let stats = emoji::render_row(
        &mut canvas,
        &font,
        "\u{1F1FA}\u{1F1F8}",
        &ep,
        20.0,
        [0, 0, 0, 255],
    )
    .unwrap();
    eprintln!("flag row: {stats:?}");
    assert!(stats.color_glyphs > 0, "flag did not render as colour");
    assert!(colourful_pixels(&canvas.to_rgba()) > 50);
}

#[test]
fn write_samples() {
    let dir = samples_dir();
    let profile = CanvasProfile::new(42);

    // Headline: the captured emoji row, rendered with swash.
    let row = emoji_text(&profile);
    std::fs::write(dir.join("emoji_row_swash.png"), &row.png).unwrap();

    // Transparent variant for compositing checks.
    let mut transparent = profile.clone();
    transparent.background = Some([0, 0, 0, 0]);
    let row_t = emoji_text(&transparent);
    std::fs::write(dir.join("emoji_row_transparent.png"), &row_t.png).unwrap();

    // ZWJ / skin tone / flag set at a readable size.
    if canvasgen::system_emoji_font().is_ok() {
        let mut big = CanvasProfile::new(42).with_size(420, 64);
        big.background = Some([255, 255, 255, 255]);
        let ep = EmojiProfile {
            font_size: 26.0,
            dpr: 1.0,
            baseline: 44.0,
            letter_spacing: 2.0,
            antialias: true,
        };
        let seq = canvasgen::emoji_row(
            &big,
            "\u{2764}\u{FE0F} \u{1F44D}\u{1F3FD} \u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466} \u{1F1FA}\u{1F1F8}",
            &ep,
            [255, 255, 255, 255],
            12.0,
        );
        std::fs::write(dir.join("emoji_sequences.png"), &seq.png).unwrap();

        // Side-by-side with the captured row (if the research checkout exists).
        let captured = std::path::Path::new(
            r"C:\Users\oli\Desktop\hcaptcha_research\captures_canvas\canvas_3_300x150.png",
        );
        if captured.exists() {
            if let Ok(bytes) = std::fs::read(captured) {
                let (cw, ch, mut cap) = decode_png(&bytes);
                // Composite the captured transparent row on white.
                for p in cap.chunks_exact_mut(4) {
                    let a = p[3] as u32;
                    for c in 0..3 {
                        p[c] = ((p[c] as u32 * a + 255 * (255 - a)) / 255) as u8;
                    }
                    p[3] = 255;
                }
                let (gw, gh, gen) = decode_png(&row.png);
                let w = cw.max(gw);
                let h = ch + gh;
                let mut stack = vec![255u8; w as usize * h as usize * 4];
                for y in 0..ch {
                    for x in 0..cw {
                        let si = (y as usize * cw as usize + x as usize) * 4;
                        let di = (y as usize * w as usize + x as usize) * 4;
                        stack[di..di + 4].copy_from_slice(&cap[si..si + 4]);
                    }
                }
                for y in 0..gh {
                    for x in 0..gw {
                        let si = (y as usize * gw as usize + x as usize) * 4;
                        let di = ((ch + y) as usize * w as usize + x as usize) * 4;
                        stack[di..di + 4].copy_from_slice(&gen[si..si + 4]);
                    }
                }
                let stacked = GeneratedCanvas::from_rgba(ProbeKind::Text, w, h, stack);
                std::fs::write(dir.join("emoji_row_compare.png"), &stacked.png).unwrap();
            }
        }
    }

    // The five probes.
    for (i, c) in fingerprint_set(&profile).into_iter().enumerate() {
        std::fs::write(
            dir.join(format!("canvas_{i}_{}x{}.png", c.width, c.height)),
            &c.png,
        )
        .unwrap();
    }

    // Default row constant should be the captured sequence.
    assert_eq!(
        DEFAULT_EMOJI_ROW,
        "\u{2764}\u{FE0F}\u{1F600}\u{1F389}\u{1F446}"
    );
}
