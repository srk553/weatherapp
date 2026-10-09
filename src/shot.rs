//! Headless SVG screenshots of the real UI (dev tool).
//!
//! Usage: `cargo run -- --screenshot [current|hourly|daily] > assets/shot-<tab>.svg`
//! Renders the actual `ui::draw` output via ratatui's `TestBackend` and converts
//! the cell buffer to an SVG that looks like a terminal screenshot.

use crate::weather::{City, Current, Day, Hour, Units, WeatherData};
use crate::{ui, App, Tab};
use ratatui::{backend::TestBackend, style::Color, Terminal};
const CELL_W: f32 = 8.0;
const CELL_H: f32 = 16.0;
const FONT_SIZE: f32 = 13.0;
const PAD: f32 = 8.0;

/// Realistic Bengaluru showcase data so screenshots look alive.
pub fn showcase_app(tab: Tab) -> App {
    let mut app = App::new(Units::Metric);
    app.cities = vec![City {
        name: "Bengaluru".into(),
        country: "India".into(),
        lat: 12.9716,
        lon: 77.5946,
    }];
    app.city_idx = 0;
    app.tab = tab;
    app.loading = false;
    app.last_updated = Some("updated 17:30".into());
    app.favorites = vec![app.cities[0].key()];

    let temps = [
        27.0, 27.0, 26.5, 26.0, 25.5, 25.0, 25.0, 26.0, 28.0, 29.5, 30.5, 31.0,
        31.5, 31.0, 30.0, 29.0, 28.5, 28.0, 27.5, 27.0, 26.5, 26.0, 26.0, 25.5,
    ];
    let hourly: Vec<Hour> = temps
        .iter()
        .enumerate()
        .map(|(i, t)| Hour {
            time: format!("{:02}:00", (17 + i) % 24),
            temp: *t,
            precip: if (10..14).contains(&i) { 30.0 } else { 5.0 },
            code: if (10..14).contains(&i) { 80 } else { 2 },
        })
        .collect();

    let days = [
        ("Fri 10/09", 2, 31.5, 21.0, 20.0),
        ("Sat 10/10", 80, 30.0, 20.5, 60.0),
        ("Sun 10/11", 61, 28.5, 20.0, 80.0),
        ("Mon 10/12", 3, 29.5, 20.0, 30.0),
        ("Tue 10/13", 1, 31.0, 20.5, 10.0),
        ("Wed 10/14", 95, 29.0, 20.0, 70.0),
        ("Thu 10/15", 2, 30.5, 21.0, 20.0),
    ];
    let daily: Vec<Day> = days
        .iter()
        .map(|(label, code, max, min, p)| Day {
            label: label.to_string(),
            code: *code,
            max: *max,
            min: *min,
            precip_max: *p,
            sunrise: "06:05".into(),
            sunset: "18:05".into(),
        })
        .collect();

    app.data = Some(WeatherData {
        city: app.cities[0].clone(),
        units: Units::Metric,
        timezone: "Asia/Kolkata".into(),
        current: Current {
            temp: 27.0,
            feels: 28.5,
            humidity: 62.0,
            code: 2,
            wind_speed: 14.0,
            wind_dir: 250.0,
            pressure: 1012.0,
            time: "17:30".into(),
        },
        hourly,
        daily,
    });
    app
}

fn fg_hex(c: Color) -> &'static str {
    match c {
        Color::Reset => "#E5E5E5",
        Color::Black => "#000000",
        Color::Red => "#E06C75",
        Color::Green => "#98C379",
        Color::Yellow => "#E5C07B",
        Color::Blue => "#61AFEF",
        Color::Magenta => "#C678DD",
        Color::Cyan => "#56B6C2",
        Color::Gray => "#ABB2BF",
        Color::DarkGray => "#5C6370",
        Color::LightRed => "#F87171",
        Color::LightGreen => "#4ADE80",
        Color::LightYellow => "#FACC15",
        Color::LightBlue => "#93C5FD",
        Color::LightMagenta => "#F0ABFC",
        Color::LightCyan => "#67E8F9",
        Color::White => "#FFFFFF",
        _ => "#E5E5E5",
    }
}

fn bg_hex(c: Color) -> Option<&'static str> {
    match c {
        Color::Reset => None,
        Color::Black => Some("#000000"),
        Color::Red => Some("#7F1D1D"),
        Color::Green => Some("#14532D"),
        Color::Yellow => Some("#713F12"),
        Color::Blue => Some("#1E3A8A"),
        Color::Magenta => Some("#701A75"),
        Color::Cyan => Some("#155E75"),
        Color::Gray => Some("#374151"),
        Color::DarkGray => Some("#1F2937"),
        Color::White => Some("#F8FAFC"),
        _ => Some("#1F2937"),
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn frame_buffer(app: &App, width: u16, height: u16) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test backend");
    terminal.draw(|f| ui::draw(f, app)).expect("draw frame");
    terminal.backend().buffer().clone()
}

fn svg_header(title: &str, width: u16, height: u16) -> String {
    let w = (width as f32 * CELL_W + PAD * 2.0) as u32;
    let h = (height as f32 * CELL_H + PAD * 2.0) as u32;
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" \
         font-family=\"'DejaVu Sans Mono','Cascadia Mono',monospace\" font-size=\"{FONT_SIZE}\">\n\
         <title>{}</title>\n\
         <rect width=\"100%\" height=\"100%\" rx=\"10\" fill=\"#10141A\"/>\n",
        esc(title)
    )
}

/// Convert one rendered frame buffer to SVG rects+texts (no wrapper).
fn frame_inner(
    buf: &ratatui::buffer::Buffer,
    width: u16,
    height: u16,
) -> String {
    use ratatui::style::Modifier;
    let mut out = String::new();
    for y in 0..height {
        let mut x: u16 = 0;
        while x < width {
            let cell = &buf[(x, y)];
            let style = cell.style();
            let key = (
                style.fg.unwrap_or(Color::Reset),
                style.bg.unwrap_or(Color::Reset),
                style.add_modifier,
                style.sub_modifier,
            );
            // Extend run while style matches.
            let mut x2 = x + 1;
            while x2 < width {
                let c2 = &buf[(x2, y)];
                let s2 = c2.style();
                if (s2.fg.unwrap_or(Color::Reset), s2.bg.unwrap_or(Color::Reset), s2.add_modifier, s2.sub_modifier) == key {
                    x2 += 1;
                } else {
                    break;
                }
            }
            let text: String = (x..x2).map(|cx| buf[(cx, y)].symbol().to_string()).collect();
            if !text.trim().is_empty() {
                let sx = PAD + x as f32 * CELL_W;
                let sy = PAD + y as f32 * CELL_H + FONT_SIZE;
                if let Some(bg) = bg_hex(key.1) {
                    out.push_str(&format!(
                        "<rect x=\"{sx:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"{bg}\"/>",
                        PAD + y as f32 * CELL_H,
                        (x2 - x) as f32 * CELL_W,
                        CELL_H
                    ));
                }
                let bold = key.2.contains(Modifier::BOLD);
                let italic = key.2.contains(Modifier::ITALIC) && !key.3.contains(Modifier::ITALIC);
                let uline = key.2.contains(Modifier::UNDERLINED);
                out.push_str(&format!(
                    "<text x=\"{sx:.1}\" y=\"{sy:.1}\" fill=\"{}\"{}>{}</text>",
                    fg_hex(key.0),
                    format!(
                        "{}{}{}",
                        if bold { " font-weight=\"bold\"" } else { "" },
                        if italic { " font-style=\"italic\"" } else { "" },
                        if uline { " text-decoration=\"underline\"" } else { "" }
                    ),
                    esc(&text)
                ));
            }
            x = x2;
        }
    }
    out
}

/// Render `tab` at `width`x`height` cells into a terminal-look SVG string.
pub fn render_svg(tab: Tab, width: u16, height: u16) -> String {
    let app = showcase_app(tab);
    let buf = frame_buffer(&app, width, height);
    format!(
        "{}{}</svg>\n",
        svg_header(&format!("SRK Weather TUI — {tab:?} tab"), width, height),
        frame_inner(&buf, width, height)
    )
}

// ---------------------------------------------------------------------------
// Animated demo recording (loops right inside the readme, no player needed)
// ---------------------------------------------------------------------------

/// Script: loading → current → hourly → daily → search typing → results →
/// help → back. Each entry is (hold_seconds, app_state).
fn demo_frames() -> Vec<(f64, App)> {
    let mut frames: Vec<(f64, App)> = Vec::new();

    // 1. Loading state (fresh app, fetch in flight).
    let mut loading = App::new(Units::Metric);
    loading.cities = vec![City {
        name: "Bengaluru".into(),
        country: "India".into(),
        lat: 12.9716,
        lon: 77.5946,
    }];
    frames.push((1.2, loading));

    // 2-4. Tabs with data.
    for (tab, hold) in [(Tab::Current, 2.5), (Tab::Hourly, 2.5), (Tab::Daily, 2.5)] {
        frames.push((hold, showcase_app(tab)));
    }

    // 5. Search typing "Paris", char by char.
    for n in 1.."Paris".len() + 1 {
        let mut a = showcase_app(Tab::Current);
        a.search_mode = true;
        a.search_input = "Paris"[..n].to_string();
        frames.push((0.3, a));
    }

    // 6. Search results.
    let mut results = showcase_app(Tab::Current);
    results.search_mode = true;
    results.search_input = "Paris".into();
    results.search_results = vec![
        City { name: "Paris".into(), country: "France".into(), lat: 48.85, lon: 2.35 },
        City { name: "Paris".into(), country: "USA (Texas)".into(), lat: 33.66, lon: -95.56 },
    ];
    frames.push((2.0, results));

    // 7. Help overlay.
    let mut help = showcase_app(Tab::Current);
    help.show_help = true;
    frames.push((2.5, help));

    // 8. Back to current.
    frames.push((1.5, showcase_app(Tab::Current)));

    frames
}

/// One looping SVG slideshow of the demo script. Plays natively on GitHub.
pub fn render_demo_svg(width: u16, height: u16) -> String {
    let frames = demo_frames();
    let total: f64 = frames.iter().map(|(h, _)| *h).sum();
    let mut out = svg_header("SRK Weather TUI demo (loops)", width, height);
    let mut t = 0.0;
    for (hold, app) in &frames {
        // Visible window [a, b] as fractions of the loop; discrete stepping.
        let (a, b) = (t / total, (t + hold) / total);
        let buf = frame_buffer(app, width, height);
        out.push_str(&format!(
            "<g visibility=\"hidden\">{}<animate attributeName=\"visibility\" \
             dur=\"{total:.2}s\" repeatCount=\"indefinite\" calcMode=\"discrete\" \
             values=\"hidden;visible;hidden\" keyTimes=\"0;{a:.4};{b:.4}\"/></g>",
            frame_inner(&buf, width, height)
        ));
        t += hold;
    }
    out.push_str("</svg>\n");
    out
}
