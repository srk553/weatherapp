# Mega-Prompt: Build a Cool Weather TUI (Rust)

Copy everything between PROMPT START and PROMPT END into any AI assistant to generate this project.

--- PROMPT START ---

You are an expert Rust TUI developer. Build a terminal user interface (TUI) app in Rust that shows
weather details from a free no-key weather API.

## Data source (must use these — no API key needed)

- Forecast: `https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}&current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,wind_speed_10m,wind_direction_10m,pressure_msl&hourly=temperature_2m,precipitation_probability,weather_code&daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max,sunrise,sunset&timezone=auto&forecast_days=7`
- Geocoding (city search): `https://geocoding-api.open-meteo.com/v1/search?name={query}&count=5&language=en&format=json`
- Docs: https://open-meteo.com/en/docs

## Tech stack

- Language: Rust (2021 edition). Deps: `ratatui` (TUI), `crossterm` (backend/events),
  `reqwest` with `blocking + json + rustls-tls` (HTTPS, no async runtime needed),
  `serde`/`serde_json` (parsing), `chrono` (times), `anyhow` (errors).
- Single binary crate. Must build with `cargo build --release` and run with `cargo run`.

## Look & feel — make it REALLY cool

- Dark neon theme: cyan headers, yellow highlights, green OK values, red alerts.
- Header bar: app title + selected city + country + local observation time.
- Tab bar: `Current | Hourly | Daily | Search | Help`.
- Current tab: big temperature readout, condition icon (unicode: ☀ 🌤 ⛅ 🌧 ⛈ ❄ 🌫),
  feels-like, humidity, wind speed + compass arrow (N/NE/E/…), pressure, sunrise/sunset.
- Hourly tab: 24h table (time, temp, precip %, icon) PLUS a line chart of temperature.
- Daily tab: 7-day table (day, icon, hi/lo with min/max bar, precip %, sunrise/sunset).
- Footer status bar, TWO sides: left = key hints + last-updated;
  **right = watermark text `SRK Master Stack`**, dim styled, ALWAYS visible
  on every screen (this is a hard requirement).
- Loading spinner state ("Fetching…") and friendly error state with retry hint.
- Responsive: must not crash on small terminals (80x24 minimum); truncate gracefully.

## Features / options (implement ALL)

1. Preset cities (7, default Bengaluru: Bengaluru, London, New York, Tokyo, Nairobi, Sydney, Rio) with quick-cycle keys `n`/`p`.
2. City search: press `/`, type name, Enter → top-5 geocoding results → arrow keys + Enter to select.
3. Units toggle: `u` switches metric (°C, km/h) ↔ imperial (°F, mph); refetch with
   `temperature_unit` + `wind_speed_unit` params.
4. Favorites: `f` toggles current city as favorite (persist to JSON file in home dir); indicator ★.
5. Manual refresh `r` + auto-refresh every 10 minutes.
6. Help overlay/screen `?` listing every keybinding.
7. CLI args: `--city "Name"` (start city), `--fahrenheit`/`--imperial` (start imperial),
   `--help` (usage). Parse with std only (no extra CLI crate needed).
8. `q` or `Esc` quits (Esc also closes overlay / cancels input first).
9. Wind direction as compass label (N, NNE, …) derived from degrees.
10. WMO weather-code → description + icon mapping (0 Clear, 1-3 Mainly clear/Overcast,
    45/48 Fog, 51-57 Drizzle, 61-67 Rain, 71-77 Snow, 80-82 Showers, 95-99 Thunderstorm).

## Keybindings table (must all work)

| Key | Action |
|-----|--------|
| `1/2/3` or `Tab` | Switch tabs (Current/Hourly/Daily) |
| `←/→` or `h/l` | Switch tabs |
| `n` / `p` | Next / previous preset city |
| `/` | Open city search |
| `Enter` | Confirm search / select result |
| `u` | Toggle °C/°F + km/h/mph |
| `f` | Toggle favorite ★ |
| `r` | Refresh now |
| `?` | Help screen |
| `q` / `Esc` | Quit (Esc first closes overlay) |

## Acceptance criteria

- `cargo build` succeeds with no warnings on stable Rust.
- App starts, fetches real data for default city, all tabs render.
- Searching "Paris" shows selectable results; selecting one updates all tabs.
- `u` visibly changes units and refetches.
- Watermark `SRK Master Stack` visible bottom-right on every screen.
- `q` always quits; terminal state restored (no broken shell after exit, even on error/panic).
- Document everything in `readme.md` (features, keys, install, usage, troubleshooting).

--- PROMPT END ---

> This prompt was used to generate the project in this folder (`src/`, `Cargo.toml`).
> See `plan.md` for the build plan and `readme.md` for user docs.
