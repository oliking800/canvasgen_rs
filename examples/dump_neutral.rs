use canvasgen::{canvas_data_urls_with_silicon, CanvasProfile, SiliconProfile, TextStyle};
fn main() {
    let ua = std::env::args().nth(1).unwrap_or_default();
    let gpu = std::env::args().nth(2).unwrap_or_default();
    let p = CanvasProfile::new(7);
    let urls = canvas_data_urls_with_silicon(&p, TextStyle::for_user_agent(&ua), SiliconProfile::for_renderer(&gpu));
    let items: Vec<String> = urls.iter().map(|u| format!("\"{u}\"")).collect();
    println!("[{}]", items.join(","));
}
