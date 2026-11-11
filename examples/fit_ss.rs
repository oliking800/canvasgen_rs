use canvasgen::{emoji, Canvas, EmojiProfile};
fn render_ss(k: u32, size: f32, base: f32, dx: f32) -> Vec<u8> {
    let font = canvasgen::system_emoji_font().unwrap();
    let mut c = Canvas::new(20 * k, 20 * k);
    let ep = EmojiProfile { font_size: size, dpr: k as f32, baseline: base, letter_spacing: 0.0, antialias: true };
    emoji::render_row(&mut c, &font, "\u{1F600}", &ep, dx, [0, 0, 0, 255]).unwrap();
    let hi = c.to_rgba();
    let mut out = vec![0u8; 1600];
    for y in 0..20u32 { for x in 0..20u32 {
        let mut acc = [0f32; 4]; let mut asum = 0f32;
        for yy in 0..k { for xx in 0..k {
            let i = (((y * k + yy) * 20 * k + x * k + xx) * 4) as usize;
            let a = hi[i + 3] as f32 / 255.0;
            acc[0] += hi[i] as f32 * a; acc[1] += hi[i + 1] as f32 * a; acc[2] += hi[i + 2] as f32 * a; asum += a;
        }}
        let o = ((y * 20 + x) * 4) as usize;
        let n = (k * k) as f32;
        if asum > 0.0 { for ch in 0..3 { out[o + ch] = (acc[ch] / asum).round() as u8; } }
        out[o + 3] = (asum / n * 255.0).round() as u8;
    }}
    out
}
fn score(a: &[u8], b: &[u8]) -> (f64, f64) {
    let mut s = 0u64; let mut n = 0;
    for (x, y) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        let d: u32 = (0..4).map(|i| (x[i] as i32 - y[i] as i32).unsigned_abs()).sum();
        s += d as u64; if d > 0 { n += 1; }
    }
    (s as f64 / 400.0 / 4.0, n as f64 / 4.0)
}
fn main() {
    let cap = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    for &k in &[1u32, 2, 4, 8] {
        let mut best = (1e9, 0.0, 0.0, 0.0, 0.0);
        let mut size = 14.5; while size <= 15.51 { let mut dx = -0.6; while dx <= 0.61 { let mut by = 14.0; while by <= 16.01 {
            let s = score(&render_ss(k, size, by, dx), &cap);
            if s.0 < best.0 { best = (s.0, s.1, size as f64, dx as f64, by as f64); }
            by += 0.125; } dx += 0.125; } size += 0.25; }
        println!("k={k} mean {:.3} diffpx% {:.1} size {} dx {:.2} base {:.2}", best.0, best.1, best.2, best.3, best.4);
    }
}
