# Build Plan — SRK Weather TUI (Rust)

## 0. Goal
A cool, keyboard-driven terminal UI showing real weather from a free no-key JSON API,
with tabs, search, units, favorites, charts, and a permanent `SRK Master Stack` watermark.

## 1. Phases

### Phase 1 — Scaffold + API (MVP)
- [x] `cargo init --bin`-style layout: `Cargo.toml` + `src/main.rs` + `src/weather.rs` + `src/ui.rs`
- [x] Deps: `ratatui 0.28`, `crossterm 0.28`, `reqwest {blocking,json,rustls-tls}`,
  `serde`, `serde_json`, `chrono`, `anyhow`
- [x] `weather.rs`: WMO code map, compass fn, `fetch_forecast()` + `geocode()` (blocking reqwest),
  structs for current/hourly/daily; unit param builders
- [x] Manual test: fetch Bengaluru, print parsed summary (before any TUI code)

### Phase 2 — App state + event loop
- [x] `City { name, country, lat, lon }` + 7 presets (Bengaluru default)
- [x] `App` struct: tab, cities, selected, units, data/loading/error, search input + results,
  favorites (persist `~/.srk_weather_favorites.json`), last_updated
- [x] Blocking-fetch pattern: draw loading → sync fetch → update state (no async runtime)
- [x] Tick loop: `crossterm::event::poll(250ms)`; auto-refresh every 10 min
- [x] Restore terminal on exit AND on panic (guard)

### Phase 3 — UI (ratatui)
- [x] Layout: header / tabs / body / status bar (split status bar L/R for watermark)
- [x] Current tab: big temp + icon + grid of details
- [x] Hourly tab: table (next 24) + `Chart` line widget
- [x] Daily tab: 7-day table with hi/lo bar
- [x] Search screen: input + selectable results
- [x] Help screen: keybinding table
- [x] Watermark `SRK Master Stack` bottom-right on EVERY screen (status bar right span)
- [x] Small-terminal safety: min-size message, no panics on 80x24

### Phase 4 — Options & polish
- [x] CLI args: `--city`, `--fahrenheit|--imperial`, `--help` (std-only parsing)
- [x] Units toggle refetch; `f` favorites persist; `r` refresh; `?` help; `q/Esc` quit
- [x] Loading + error states with retry hint
- [x] `cargo build` clean (zero warnings), smoke test `--help` + release build

## 2. File layout

```text
srk-weather-tui/
├── Cargo.toml        # binary crate, deps listed above
├── prompt.md         # generation prompt (spec)
├── plan.md           # this file
├── readme.md         # user docs
└── src/
    ├── main.rs       # App state, event loop, CLI args, terminal setup
    ├── weather.rs    # API client, structs, WMO map, compass, units
    └── ui.rs         # all ratatui rendering (header/tabs/body/status+watermark)
```

## 3. API mapping

| App need | Request |
|----------|---------|
| Current + hourly + daily | `GET /v1/forecast` (params in prompt.md) + `temperature_unit`/`wind_speed_unit` |
| City search | `GET geocoding-api…/v1/search?name=&count=5` → map to `City` |
| Timezone | `timezone=auto` → display `utc_offset_seconds`-adjusted local times |

## 4. State machine

```text
Startup → Loading → Ready ⇄ (Tab switch | Search → Results → Select → Loading → Ready)
Ready → Error (fetch fail) → [r] Retry → Loading
Any → Quit (q) → restore terminal
```

## 5. Key handling matrix

`1/2/3`, `Tab`, `←/→`/`h/l` tabs · `n`/`p` preset city · `/` search · `Enter` confirm · `u` units · `f` fav ·
`r` refresh · `?` help · `Esc` close-then-quit · `q` quit · search mode: chars/Backspace nav.

## 6. Risks & mitigations

- No network → friendly error + retry, never crash. ✅
- OpenSSL missing → use `rustls-tls` (no system TLS). ✅
- Panic leaves broken terminal → terminal-guard restores cooked mode + screen. ✅
- Tiny terminal → min-size guard message. ✅
- Favorites file unwritable → in-memory fallback, warn in status. ✅
