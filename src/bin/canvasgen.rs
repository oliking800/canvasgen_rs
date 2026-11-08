//! `canvasgen` command line interface.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

use canvasgen::{
    canvas_data_urls, fingerprint_set, pngout, render_probe, system_font_info, CanvasProfile,
    ProbeKind, CANVAS_CTXS_JSON, VERSION,
};

#[derive(Parser, Debug)]
#[command(
    name = "canvasgen",
    version,
    about = "Generate plausible browser canvas fingerprints (2D + WebGL probes)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Write the five probe PNGs (same mix/sizes as hCaptcha's canvasDataURLs).
    Probes {
        /// Output directory.
        #[arg(long, short)]
        out: PathBuf,
        /// Deterministic seed.
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Font file (TTF/OTF/TTC). Defaults to the platform colour emoji font.
        #[arg(long)]
        font: Option<PathBuf>,
        /// Face index inside a font collection.
        #[arg(long, default_value_t = 0)]
        font_index: u32,
        /// Device pixel ratio used for rasterization.
        #[arg(long, default_value_t = 1.0)]
        dpr: f32,
        /// Override the emoji row.
        #[arg(long)]
        text: Option<String>,
        /// Font size for `--text` (default 10).
        #[arg(long, default_value_t = 10.0)]
        font_size: f32,
        /// Baseline for `--text` (default 70).
        #[arg(long, default_value_t = 70.0)]
        baseline: f32,
    },
    /// Print one data URL.
    Url {
        /// Probe kind.
        #[arg(long, value_enum)]
        kind: Kind,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long)]
        font: Option<PathBuf>,
        #[arg(long, default_value_t = 0)]
        font_index: u32,
        #[arg(long, default_value_t = 1.0)]
        dpr: f32,
        #[arg(long)]
        text: Option<String>,
        /// Font size for `--text` (default 10).
        #[arg(long, default_value_t = 10.0)]
        font_size: f32,
        /// Baseline for `--text` (default 70).
        #[arg(long, default_value_t = 70.0)]
        baseline: f32,
    },
    /// Print the five data URLs as a JSON array.
    Urls {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long)]
        font: Option<PathBuf>,
        #[arg(long, default_value_t = 0)]
        font_index: u32,
        #[arg(long, default_value_t = 1.0)]
        dpr: f32,
    },
    /// Print `{"canvasDataURLs": [...], "canvasCtxs": [...]}` for env827.
    Env {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long)]
        font: Option<PathBuf>,
        #[arg(long, default_value_t = 0)]
        font_index: u32,
        #[arg(long, default_value_t = 1.0)]
        dpr: f32,
    },
    /// Show the detected colour emoji font and crate version.
    Info,
    /// Apply per-device micro-variation to five data URLs (one per line on
    /// stdin; writes five lines).
    Vary {
        #[arg(long, default_value_t = 1)]
        seed: u64,
    },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Kind {
    Text,
    Triangle,
    Small,
    Abstract,
}

impl Kind {
    fn probe(self) -> ProbeKind {
        match self {
            Kind::Text => ProbeKind::Text,
            Kind::Triangle => ProbeKind::Triangle,
            Kind::Small => ProbeKind::Small,
            Kind::Abstract => ProbeKind::Abstract,
        }
    }
}

fn profile(
    seed: u64,
    dpr: f32,
    font: Option<&Path>,
    font_index: u32,
) -> Result<CanvasProfile, String> {
    let mut p = CanvasProfile::new(seed);
    p.dpr = dpr;
    p.font_index = font_index;
    if let Some(path) = font {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        p.font = Some(bytes);
    }
    Ok(p)
}

fn write_png(dir: &Path, name: &str, bytes: &[u8]) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(name);
    std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::Info => {
            println!("canvasgen {VERSION}");
            println!("emoji font: {}", system_font_info());
            println!("probes: {}", canvasgen::FINGERPRINT_PROBES.len());
            Ok(())
        }
        Command::Vary { seed } => {
            use std::io::Read;
            let mut input = String::new();
            std::io::stdin()
                .read_to_string(&mut input)
                .map_err(|e| format!("stdin: {e}"))?;
            let lines: Vec<&str> = input
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .collect();
            if lines.len() != 5 {
                return Err(format!(
                    "vary expects 5 data URLs (one per line), got {}",
                    lines.len()
                ));
            }
            let vary = canvasgen::VaryProfile::from_seed(seed);
            for (i, line) in lines.iter().enumerate() {
                let kind = canvasgen::FINGERPRINT_PROBES[i].0;
                println!("{}", canvasgen::vary_canvas_data_url(line, kind, &vary)?);
            }
            Ok(())
        }
        Command::Probes {
            out,
            seed,
            font,
            font_index,
            dpr,
            text,
            font_size,
            baseline,
        } => {
            let base = profile(seed, dpr, font.as_deref(), font_index)?;
            let mut entries = Vec::new();
            for (i, c) in fingerprint_set(&base).into_iter().enumerate() {
                let name = format!("canvas_{i}_{}x{}.png", c.width, c.height);
                write_png(&out, &name, &c.png)?;
                entries.push(format!(
                    "{{\"index\":{i},\"kind\":\"{}\",\"width\":{},\"height\":{},\"file\":\"{}\",\"bytes\":{},\"data_url_len\":{}}}",
                    c.kind.as_str(),
                    c.width,
                    c.height,
                    name,
                    c.png.len(),
                    c.data_url.len()
                ));
            }
            let manifest = format!(
                "{{\"seed\":{seed},\"dpr\":{dpr},\"count\":5,\"probes\":[{}]}}",
                entries.join(",")
            );
            write_png(&out, "manifest.json", manifest.as_bytes())?;

            if let Some(text) = text {
                // Extra debugging render of a custom row, never part of the set.
                let mut p = base.with_size(300, 150);
                p.seed = seed;
                let ep = canvasgen::EmojiProfile {
                    dpr,
                    font_size,
                    baseline,
                    ..Default::default()
                };
                let canvas = canvasgen::emoji_row(&p, &text, &ep, [255, 255, 255, 255], 12.0);
                write_png(&out, "custom_row.png", &canvas.png)?;
            } else {
                let _ = &out;
            }
            println!("{}", out.display());
            Ok(())
        }
        Command::Url {
            kind,
            seed,
            font,
            font_index,
            dpr,
            text,
            font_size,
            baseline,
        } => {
            let mut base = profile(seed, dpr, font.as_deref(), font_index)?;
            let generated = if let Some(text) = text {
                base.width = 300;
                base.height = 150;
                let ep = canvasgen::EmojiProfile {
                    dpr,
                    font_size,
                    baseline,
                    ..Default::default()
                };
                canvasgen::emoji_row(&base, &text, &ep, [255, 255, 255, 255], 12.0)
            } else {
                render_probe(kind.probe(), &base)
            };
            println!("{}", generated.data_url);
            Ok(())
        }
        Command::Urls {
            seed,
            font,
            font_index,
            dpr,
        } => {
            let p = profile(seed, dpr, font.as_deref(), font_index)?;
            let urls: Vec<String> = canvas_data_urls(&p)
                .into_iter()
                .map(|u| pngout::json_escape(&u))
                .collect();
            println!("[{}]", urls.join(","));
            Ok(())
        }
        Command::Env {
            seed,
            font,
            font_index,
            dpr,
        } => {
            let p = profile(seed, dpr, font.as_deref(), font_index)?;
            let urls: Vec<String> = canvas_data_urls(&p)
                .into_iter()
                .map(|u| pngout::json_escape(&u))
                .collect();
            println!(
                "{{\"canvasDataURLs\":[{}],\"canvasCtxs\":{CANVAS_CTXS_JSON}}}",
                urls.join(",")
            );
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("canvasgen: {e}");
            ExitCode::FAILURE
        }
    }
}
