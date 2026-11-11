use canvasgen::{canvas_data_urls_with, CanvasProfile, TextStyle};
fn main() {
    let ua = std::env::args().nth(1).unwrap_or_default();
    let p = CanvasProfile::new(7);
    let urls = canvas_data_urls_with(&p, TextStyle::for_user_agent(&ua));
    let items: Vec<String> = urls.iter().map(|u| format!("\"{u}\"")).collect();
    println!("[{}]", items.join(","));
}
