//! SRK Weather TUI — cool terminal weather app.
//!
//! Run: `cargo run` · `cargo run -- --city Paris --fahrenheit` · `cargo run -- --help`

mod ui;
mod weather;
/// Headless SVG screenshots (`--screenshot`), not part of the TUI itself.
mod shot;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io::Stdout,
    time::{Duration, Instant},
};
use weather::{geocode, City, Units, WeatherData};

// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Current,
    Hourly,
    Daily,
}

impl Tab {
    fn index(self) -> usize {
        match self {
            Tab::Current => 0,
            Tab::Hourly => 1,
            Tab::Daily => 2,
        }
    }
    fn next(self) -> Self {
        match self {
            Tab::Current => Tab::Hourly,
            Tab::Hourly => Tab::Daily,
            Tab::Daily => Tab::Current,
        }
    }
    fn prev(self) -> Self {
        match self {
            Tab::Current => Tab::Daily,
            Tab::Hourly => Tab::Current,
            Tab::Daily => Tab::Hourly,
        }
    }
}

// ---------------------------------------------------------------------------

pub struct App {
    pub cities: Vec<City>,
    pub city_idx: usize,
    pub custom_city: Option<City>,
    pub units: Units,
    pub tab: Tab,
    pub data: Option<WeatherData>,
    pub loading: bool,
    pub error: Option<String>,
    pub last_updated: Option<String>,
    pub last_fetch: Option<Instant>,
    pub show_help: bool,
    pub search_mode: bool,
    pub searching: bool,
    pub search_input: String,
    pub search_results: Vec<City>,
    pub search_selected: usize,
    pub favorites: Vec<String>,
}

impl App {
    fn new(units: Units) -> Self {
        Self {
            cities: weather::preset_cities(),
            city_idx: 0,
            custom_city: None,
            units,
            tab: Tab::Current,
            data: None,
            loading: true,
            error: None,
            last_updated: None,
            last_fetch: None,
            show_help: false,
            search_mode: false,
            searching: false,
            search_input: String::new(),
            search_results: Vec::new(),
            search_selected: 0,
            favorites: load_favorites(),
        }
    }

    pub fn current_city(&self) -> &City {
        self.custom_city.as_ref().unwrap_or(&self.cities[self.city_idx])
    }

    pub fn is_favorite(&self, city: &City) -> bool {
        self.favorites.iter().any(|k| k == &city.key())
    }

    fn toggle_favorite(&mut self) {
        let key = self.current_city().key();
        if self.favorites.iter().any(|k| k == &key) {
            self.favorites.retain(|k| k != &key);
        } else {
            self.favorites.push(key);
        }
        save_favorites(&self.favorites);
    }

    fn use_preset(&mut self, idx: usize) {
        self.city_idx = idx % self.cities.len();
        self.custom_city = None;
        self.fetch();
    }

    fn cycle_city(&mut self, dir: i32) {
        let n = self.cities.len() as i32;
        let cur = if self.custom_city.is_some() {
            if dir > 0 { 0 } else { n - 1 }
        } else {
            (self.city_idx as i32 + dir + n) % n
        };
        self.use_preset(cur as usize);
    }

    fn fetch(&mut self) {
        self.loading = true;
        self.error = None;
        let city = self.current_city().clone();
        match weather::fetch_forecast(&city, self.units) {
            Ok(data) => {
                self.data = Some(data);
                self.last_updated = Some(
                    chrono::Local::now().format("updated %H:%M").to_string(),
                );
                self.last_fetch = Some(Instant::now());
            }
            Err(e) => {
                self.error = Some(format!("{:#}", e));
                self.last_fetch = Some(Instant::now());
            }
        }
        self.loading = false;
    }

    fn run_search(&mut self) {
        let q = self.search_input.trim().to_string();
        if q.is_empty() {
            return;
        }
        self.searching = true;
        self.search_results.clear();
        match geocode(&q) {
            Ok(list) => self.search_results = list,
            Err(e) => self.error = Some(format!("search failed: {:#}", e)),
        }
        self.searching = false;
        self.search_selected = 0;
    }

    fn select_search_result(&mut self) {
        if let Some(c) = self.search_results.get(self.search_selected).cloned() {
            self.custom_city = Some(c);
            self.search_mode = false;
            self.search_input.clear();
            self.search_results.clear();
            self.fetch();
        }
    }
}

// ------------------------------------------------------------- favorites ---

fn favorites_path() -> Option<std::path::PathBuf> {
    std::env::var("HOME").ok().map(|h| {
        std::path::PathBuf::from(h).join(".srk_weather_favorites.json")
    })
}

fn load_favorites() -> Vec<String> {
    favorites_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_favorites(favs: &[String]) {
    if let Some(p) = favorites_path() {
        let _ = std::fs::write(p, serde_json::to_string_pretty(favs).unwrap_or_default());
    }
}

// ------------------------------------------------------------------ CLI ---

fn print_help() {
    println!(
        "SRK Weather TUI — terminal weather app\n\
         \n\
         Usage: srk-weather-tui [--city \"Name\"] [--fahrenheit|--imperial] [--help]\n\
         \n\
         Dev: srk-weather-tui --screenshot [current|hourly|daily|demo]  (SVG to stdout)\n\
         \n\
         Keys: Tab/1/2/3 tabs · n/p city · / search · u units · f favorite ·\n\
         \u{20}     r refresh · ? help · q quit\n\
         \n\
         Data: free no-key weather API. Watermark: SRK Master Stack."
    );
}

// ----------------------------------------------------------------- main ---

fn main() -> Result<()> {
    // ---- CLI args (std only) ----
    let args: Vec<String> = std::env::args().collect();
    let mut start_city: Option<String> = None;
    let mut units = Units::Metric;
    let mut screenshot: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--screenshot" => {
                // Optional value: next arg unless it looks like a flag.
                match args.get(i + 1) {
                    Some(next) if !next.starts_with('-') => {
                        screenshot = Some(next.clone());
                        i += 1;
                    }
                    _ => screenshot = Some("current".into()),
                }
            }
            s if s.starts_with("--screenshot=") => {
                screenshot = Some(s.trim_start_matches("--screenshot=").to_string());
            }
            "--fahrenheit" | "--imperial" | "-f" => units = Units::Imperial,
            "--city" => {
                i += 1;
                if i < args.len() {
                    start_city = Some(args[i].clone());
                }
            }
            a if a.starts_with("--city=") => {
                start_city = Some(a.trim_start_matches("--city=").to_string())
            }
            _ => {}
        }
        i += 1;
    }

    let mut app = App::new(units);

    // Headless screenshots: print SVG of the real UI, no terminal needed.
    if let Some(which) = screenshot {
        match which.as_str() {
            "demo" => print!("{}", shot::render_demo_svg(100, 30)),
            "hourly" => print!("{}", shot::render_svg(Tab::Hourly, 100, 30)),
            "daily" => print!("{}", shot::render_svg(Tab::Daily, 100, 30)),
            _ => print!("{}", shot::render_svg(Tab::Current, 100, 30)),
        }
        return Ok(());
    }

    // Resolve --city: preset match first, else geocode first hit (offline-safe).
    if let Some(name) = start_city {
        let needle = name.to_lowercase();
        if let Some(idx) = app
            .cities
            .iter()
            .position(|c| c.name.to_lowercase().contains(&needle))
        {
            app.city_idx = idx;
        } else if let Ok(list) = geocode(&name) {
            if let Some(hit) = list.into_iter().next() {
                app.custom_city = Some(hit);
            }
        }
    }

    // ---- terminal setup (restored on exit AND panic) ----
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        prev_hook(info);
    }));

    let backend = CrosstermBackend::new(stdout);
    let mut terminal: Terminal<CrosstermBackend<Stdout>> = Terminal::new(backend)?;

    // Initial fetch (shows loading first).
    terminal.draw(|f| ui::draw(f, &app))?;
    app.fetch();

    let res = run_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    res
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> Result<()> {
    const AUTO_REFRESH: Duration = Duration::from_secs(600);
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        // Auto-refresh every 10 min when idle.
        if !app.loading
            && app
                .last_fetch
                .map(|t| t.elapsed() > AUTO_REFRESH)
                .unwrap_or(false)
        {
            app.fetch();
            continue;
        }

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            break;
        }

        // ---- search input mode ----
        if app.search_mode {
            match key.code {
                KeyCode::Esc => {
                    app.search_mode = false;
                    app.search_input.clear();
                    app.search_results.clear();
                }
                KeyCode::Enter => {
                    if app.search_results.is_empty() {
                        app.run_search();
                    } else {
                        app.select_search_result();
                    }
                }
                KeyCode::Backspace => {
                    app.search_input.pop();
                }
                KeyCode::Up => {
                    if !app.search_results.is_empty() {
                        app.search_selected = app
                            .search_selected
                            .saturating_sub(1)
                            .min(app.search_results.len() - 1);
                    }
                }
                KeyCode::Down => {
                    if !app.search_results.is_empty() {
                        app.search_selected =
                            (app.search_selected + 1) % app.search_results.len();
                    }
                }
                KeyCode::Char(c) => app.search_input.push(c),
                _ => {}
            }
            continue;
        }

        // ---- help overlay ----
        if app.show_help {
            match key.code {
                KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => {
                    app.show_help = false;
                    if key.code == KeyCode::Char('q') {
                        break;
                    }
                }
                _ => {}
            }
            continue;
        }

        // ---- normal mode ----
        match key.code {
            KeyCode::Char('q') => break,
            KeyCode::Esc => break,
            KeyCode::Char('?') => app.show_help = true,
            KeyCode::Char('/') => {
                app.search_mode = true;
                app.search_input.clear();
                app.search_results.clear();
                app.search_selected = 0;
            }
            KeyCode::Char('u') => {
                app.units = app.units.toggle();
                app.fetch();
            }
            KeyCode::Char('f') => app.toggle_favorite(),
            KeyCode::Char('r') => app.fetch(),
            KeyCode::Char('n') => app.cycle_city(1),
            KeyCode::Char('p') => app.cycle_city(-1),
            KeyCode::Char('1') => app.tab = Tab::Current,
            KeyCode::Char('2') => app.tab = Tab::Hourly,
            KeyCode::Char('3') => app.tab = Tab::Daily,
            KeyCode::Tab => app.tab = app.tab.next(),
            KeyCode::BackTab => app.tab = app.tab.prev(),
            KeyCode::Right | KeyCode::Char('l') => app.tab = app.tab.next(),
            KeyCode::Left | KeyCode::Char('h') => app.tab = app.tab.prev(),
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn sample_app() -> App {
        let mut app = App::new(Units::Metric);
        app.loading = false;
        app.data = Some(WeatherData {
            city: app.cities[0].clone(),
            units: Units::Metric,
            timezone: "Asia/Kolkata".into(),
            current: weather::Current {
                temp: 18.0,
                feels: 17.0,
                humidity: 70.0,
                code: 2,
                wind_speed: 10.0,
                wind_dir: 200.0,
                pressure: 1013.0,
                time: "17:30".into(),
            },
            hourly: vec![weather::Hour {
                time: "17:00".into(),
                temp: 18.0,
                precip: 10.0,
                code: 2,
            }],
            daily: vec![weather::Day {
                label: "Fri 10/09".into(),
                code: 2,
                max: 19.0,
                min: 12.0,
                precip_max: 20.0,
                sunrise: "06:50".into(),
                sunset: "18:40".into(),
            }],
        });
        app.last_updated = Some("updated 17:30".into());
        app
    }

    #[test]
    fn renders_header_data_and_watermark() {
        let app = sample_app();
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui::draw(f, &app)).unwrap();
        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(content.contains("Bengaluru"), "header shows city");
        assert!(
            content.contains("SRK Master Stack"),
            "watermark on every screen"
        );
        assert!(content.contains("18"), "temperature shown");
    }

    #[test]
    fn watermark_present_on_all_tabs_and_overlays() {
        let mut app = sample_app();
        for tab in [Tab::Current, Tab::Hourly, Tab::Daily] {
            app.tab = tab;
            for overlay in [false, true] {
                app.show_help = overlay;
                let backend = TestBackend::new(100, 30);
                let mut terminal = Terminal::new(backend).unwrap();
                terminal.draw(|f| ui::draw(f, &app)).unwrap();
                let content: String = terminal
                    .backend()
                    .buffer()
                    .content()
                    .iter()
                    .map(|c| c.symbol().to_string())
                    .collect();
                assert!(
                    content.contains("SRK Master Stack"),
                    "watermark on tab {:?} help={}",
                    tab,
                    overlay
                );
            }
        }
        // Search overlay too.
        app.show_help = false;
        app.search_mode = true;
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui::draw(f, &app)).unwrap();
        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(content.contains("SRK Master Stack"), "watermark on search");
    }
}
