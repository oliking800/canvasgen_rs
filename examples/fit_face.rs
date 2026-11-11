//! Fit the 20x20 face's origin/size/gamma to a real capture (raw RGBA, 1600 bytes).
use canvasgen::{emoji, Canvas, CanvasProfile, EmojiProfile};
fn render(p: &CanvasProfile, size: f32, base: f32, dx: f32, gamma: f32) -> Vec<u8> {
    let _ = p; let font = canvasgen::system_emoji_font().unwrap();
    let mut c = Canvas::new(20, 20);
    let ep = EmojiProfile { font_size: size, dpr: 1.0, baseline: base, letter_spacing: 0.0, antialias: true };
    emoji::render_row(&mut c, &font, "\u{1F600}", &ep, dx, [0, 0, 0, 255]).unwrap();
    let mut rgba = c.to_rgba();
    if (gamma - 1.0).abs() > 1e-4 {
        for px in rgba.chunks_exact_mut(4) {
            px[3] = (255.0 * (px[3] as f32 / 255.0).powf(gamma)).round() as u8;
        }
    }
    rgba
}
fn score(a: &[u8], b: &[u8]) -> (f64, f64) {
    let mut s = 0u64; let mut n = 0;
    for (x, y) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        let d: u32 = (0..4).map(|i| (x[i] as i32 - y[i] as i32).unsigned_abs()).sum();
        s += d as u64; if d > 0 { n += 1; }
    }
    (s as f64 / 1600.0 / 4.0, n as f64 / 4.0)
}
fn main() {
    let cap = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let p = CanvasProfile::new(7);
    let base = score(&render(&p, 15.0, 15.0, 0.0, 1.0), &cap);
    println!("baseline mean {:.3} diffpx% {:.1}", base.0, base.1);
    let mut best = (1e9, 0.0, 0.0, 0.0, 0.0, 0.0);
    let mut size = 14.0;
    while size <= 16.01 {
        let mut dx = -1.0;
        while dx <= 1.01 {
            let mut base_y = 13.5;
            while base_y <= 16.51 {
                for &g in &[0.9f32, 1.0, 1.1] {
                    let s = score(&render(&p, size, base_y, dx, g), &cap);
                    if s.0 < best.0 { best = (s.0, s.1, size as f64, dx as f64, base_y as f64, g as f64); }
                }
                base_y += 0.1;
            }
            dx += 0.1;
        }
        size += 0.25;
    }
    println!("best mean {:.3} diffpx% {:.1} size {} dx {:.2} baseline {:.2} gamma {}", best.0, best.1, best.2, best.3, best.4, best.5);
}
