//! All ratatui rendering. The watermark lives in the status bar (bottom-right).

use crate::weather::{compass, wmo_info};
use crate::{App, Tab};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph, Row, Table, Tabs},
    Frame,
};

/// Watermark shown bottom-right on EVERY screen (hard requirement).
pub const WATERMARK: &str = "SRK Master Stack";

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    if area.width < 60 || area.height < 18 {
        let msg = Paragraph::new(vec![
            Line::from("Terminal too small — please resize to at least 80x24."),
            Line::from(format!("Current: {}x{}", area.width, area.height)),
        ])
        .block(Block::default().title(" SRK Weather ").borders(Borders::ALL));
        f.render_widget(msg, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Length(3), // tabs
            Constraint::Min(8),    // body
            Constraint::Length(1), // status bar (with watermark)
        ])
        .split(area);

    draw_header(f, chunks[0], app);
    draw_tabs(f, chunks[1], app);

    if app.show_help {
        draw_help(f, chunks[2]);
    } else if app.search_mode {
        draw_search(f, chunks[2], app);
    } else {
        match app.tab {
            Tab::Current => draw_current(f, chunks[2], app),
            Tab::Hourly => draw_hourly(f, chunks[2], app),
            Tab::Daily => draw_daily(f, chunks[2], app),
        }
    }

    draw_status(f, chunks[3], app);
}

// ---------------------------------------------------------------- header ---

fn draw_header(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let city = app.current_city();
    let star = if app.is_favorite(city) { " ★" } else { "" };
    let title = format!(
        " ☀ SRK Weather  │  {}{}  │  {}  │  {} ",
        city.label(),
        star,
        app.units.name(),
        app.last_updated.as_deref().unwrap_or("not updated yet")
    );
    let p = Paragraph::new(Line::from(vec![Span::styled(
        title,
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );
    f.render_widget(p, area);
}

// ------------------------------------------------------------------ tabs ---

fn draw_tabs(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let titles = [" Current ", " Hourly ", " Daily "]
        .iter()
        .map(|t| Line::from(*t))
        .collect::<Vec<_>>();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(" Views "))
        .select(app.tab.index())
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::raw("│"));
    f.render_widget(tabs, area);
}

// --------------------------------------------------------------- current ---

fn draw_current(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let Some(data) = &app.data else {
        f.render_widget(loading_or_error(app), area);
        return;
    };
    let c = &data.current;
    let (icon, desc) = wmo_info(c.code);
    let u = data.units.temp_label();
    let wu = data.units.wind_label();

    let headline = Line::from(vec![
        Span::styled(
            format!(" {} {:.0}{} ", icon, c.temp, u),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{}  (feels {:.0}{})", desc, c.feels, u),
            Style::default().fg(Color::White),
        ),
    ]);

    let rows = vec![
        Row::new(vec![
            format!("Humidity"),
            format!("{:.0} %", c.humidity),
        ]),
        Row::new(vec![
            format!("Wind"),
            if c.wind_speed.is_nan() || c.wind_dir.is_nan() {
                "--".into()
            } else {
                format!("{:.0} {} {}", c.wind_speed, wu, compass(c.wind_dir))
            },
        ]),
        Row::new(vec![
            format!("Pressure"),
            format!("{:.0} hPa", c.pressure),
        ]),
        Row::new(vec![
            format!("Observed"),
            format!("{} ({})", c.time, data.timezone),
        ]),
    ];
    if let Some(day0) = data.daily.first() {
        let _ = day0; // sunrise/sunset shown on Daily tab; keep Current compact
    }

    let table = Table::new(rows, [Constraint::Length(12), Constraint::Min(10)])
        .block(Block::default().borders(Borders::ALL).title(" Now "))
        .column_spacing(2);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(6)])
        .split(area);
    f.render_widget(
        Paragraph::new(headline).block(Block::default().borders(Borders::ALL)),
        chunks[0],
    );
    f.render_widget(table, chunks[1]);
}

// ---------------------------------------------------------------- hourly ---

fn draw_hourly(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let Some(data) = &app.data else {
        f.render_widget(loading_or_error(app), area);
        return;
    };
    if data.hourly.is_empty() {
        f.render_widget(
            Paragraph::new("No hourly data returned by the API.")
                .block(Block::default().borders(Borders::ALL).title(" Hourly ")),
            area,
        );
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    // Chart of next-24h temps.
    let pts: Vec<(f64, f64)> = data
        .hourly
        .iter()
        .enumerate()
        .map(|(i, h)| (i as f64, h.temp))
        .collect();
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for h in &data.hourly {
        if !h.temp.is_nan() {
            lo = lo.min(h.temp);
            hi = hi.max(h.temp);
        }
    }
    if lo > hi {
        lo = 0.0;
        hi = 1.0;
    }
    let pad = ((hi - lo) * 0.2).max(1.0);
    let dataset = Dataset::default()
        .name(format!("24h {}", data.units.temp_label()))
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Cyan))
        .data(&pts);
    let chart = Chart::new(vec![dataset])
        .block(Block::default().borders(Borders::ALL).title(" Temp next 24h "))
        .x_axis(
            Axis::default()
                .bounds([0.0, (pts.len().saturating_sub(1)) as f64])
                .labels([Span::raw("now"), Span::raw("+12h"), Span::raw("+24h")]),
        )
        .y_axis(
            Axis::default()
                .bounds([lo - pad, hi + pad])
                .labels([
                    Span::raw(format!("{:.0}", lo - pad)),
                    Span::raw(format!("{:.0}", (lo + hi) / 2.0)),
                    Span::raw(format!("{:.0}", hi + pad)),
                ]),
        );
    f.render_widget(chart, chunks[0]);

    let rows: Vec<Row> = data
        .hourly
        .iter()
        .map(|h| {
            let (icon, _) = wmo_info(h.code);
            Row::new(vec![
                h.time.clone(),
                format!("{:.0}{}", h.temp, data.units.temp_label()),
                format!("{:.0}%", h.precip),
                icon.to_string(),
            ])
        })
        .collect();
    let table = Table::new(
        rows,
        [
            Constraint::Length(7),
            Constraint::Length(9),
            Constraint::Length(7),
            Constraint::Length(4),
        ],
    )
    .header(
        Row::new(vec!["Time", "Temp", "Rain", "Sky"])
            .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(Block::default().borders(Borders::ALL).title(" Hour by hour "))
    .column_spacing(1);
    f.render_widget(table, chunks[1]);
}

// ----------------------------------------------------------------- daily ---

/// 10-cell hi/lo bar between week min and max.
fn range_bar(min: f64, max: f64, week_min: f64, week_max: f64) -> String {
    const W: usize = 10;
    if min.is_nan() || max.is_nan() || week_max <= week_min {
        return "─".repeat(W);
    }
    let span = week_max - week_min;
    let a = (((min - week_min) / span) * W as f64).floor() as usize;
    let b = (((max - week_min) / span) * W as f64).ceil() as usize;
    (0..W)
        .map(|i| if i >= a.min(W) && i < b.min(W + 1).max(1) { '█' } else { '─' })
        .collect()
}

fn draw_daily(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let Some(data) = &app.data else {
        f.render_widget(loading_or_error(app), area);
        return;
    };
    if data.daily.is_empty() {
        f.render_widget(
            Paragraph::new("No daily data returned by the API.")
                .block(Block::default().borders(Borders::ALL).title(" Daily ")),
            area,
        );
        return;
    }

    let (mut wmin, mut wmax) = (f64::INFINITY, f64::NEG_INFINITY);
    for d in &data.daily {
        if !d.min.is_nan() {
            wmin = wmin.min(d.min);
        }
        if !d.max.is_nan() {
            wmax = wmax.max(d.max);
        }
    }

    let rows: Vec<Row> = data
        .daily
        .iter()
        .map(|d| {
            let (icon, _) = wmo_info(d.code);
            Row::new(vec![
                d.label.clone(),
                icon.to_string(),
                format!("{:.0}°", d.max),
                format!("{:.0}°", d.min),
                range_bar(d.min, d.max, wmin, wmax),
                format!("{:.0}%", d.precip_max),
                format!("{}–{}", d.sunrise, d.sunset),
            ])
        })
        .collect();
    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(12),
            Constraint::Length(6),
            Constraint::Min(12),
        ],
    )
    .header(
        Row::new(vec!["Day", "Sky", "Hi", "Lo", "Range", "Rain", "Sun ↑↓"])
            .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(Block::default().borders(Borders::ALL).title(" 7-day "))
    .column_spacing(1);
    f.render_widget(table, area);
}

// ---------------------------------------------------------------- search ---

fn draw_search(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    let input = Paragraph::new(app.search_input.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Search city (Enter=go, Esc=cancel) "),
    );
    f.render_widget(input, chunks[0]);
    f.set_cursor_position((
        chunks[0].x + 1 + app.search_input.len() as u16,
        chunks[0].y + 1,
    ));

    if app.searching {
        f.render_widget(
            Paragraph::new("Searching…")
                .block(Block::default().borders(Borders::ALL).title(" Results ")),
            chunks[1],
        );
        return;
    }
    if app.search_results.is_empty() {
        f.render_widget(
            Paragraph::new("Type a city name and press Enter. Example: Paris")
                .block(Block::default().borders(Borders::ALL).title(" Results ")),
            chunks[1],
        );
        return;
    }
    let rows: Vec<Row> = app
        .search_results
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let marker = if i == app.search_selected { "▶" } else { " " };
            let style = if i == app.search_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(vec![
                marker.to_string(),
                c.name.clone(),
                c.country.clone(),
                format!("{:.2},{:.2}", c.lat, c.lon),
            ])
            .style(style)
        })
        .collect();
    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Min(12),
            Constraint::Min(10),
            Constraint::Length(18),
        ],
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Results (↑/↓ + Enter) "),
    );
    f.render_widget(table, chunks[1]);
}

// ------------------------------------------------------------------ help ---

fn draw_help(f: &mut Frame, area: ratatui::layout::Rect) {
    let text = vec![
        Line::from("Keybindings (? or Esc closes)"),
        Line::from(""),
        Line::from(" 1/2/3  Tab  ←/→  h/l ..... switch tabs"),
        Line::from(" n / p ...................... next / previous city"),
        Line::from(" / .......................... search city"),
        Line::from(" Enter ...................... confirm / select result"),
        Line::from(" u .......................... toggle Metric ↔ Imperial"),
        Line::from(" f .......................... toggle favorite ★"),
        Line::from(" r .......................... refresh now"),
        Line::from(" ? .......................... this help"),
        Line::from(" q / Esc .................... quit (Esc closes overlay first)"),
        Line::from(""),
        Line::from("CLI: --city \"Name\"  --fahrenheit|--imperial  --help"),
    ];
    f.render_widget(
        Paragraph::new(text).block(Block::default().borders(Borders::ALL).title(" Help ")),
        area,
    );
}

// ---------------------------------------------------------------- states ---

fn loading_or_error(app: &App) -> Paragraph<'_> {
    if app.loading {
        Paragraph::new("Fetching weather …")
            .block(Block::default().borders(Borders::ALL).title(" Weather "))
    } else if let Some(err) = &app.error {
        Paragraph::new(vec![
            Line::from(Span::styled(
                "Couldn't load weather:",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(err.clone()),
            Line::from("Press r to retry, / to search another city."),
        ])
        .block(Block::default().borders(Borders::ALL).title(" Error "))
    } else {
        Paragraph::new("No data yet — press r to refresh.")
            .block(Block::default().borders(Borders::ALL).title(" Weather "))
    }
}

// ---------------------------------------------------------------- status ---

fn draw_status(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let left = if app.search_mode {
        "type + Enter: search │ Esc: cancel"
    } else if app.show_help {
        "?:close help │ q:quit"
    } else {
        "Tab:tabs n/p:city /:search u:units f:★ r:refresh ?:help q:quit"
    };
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(10), Constraint::Length(WATERMARK.len() as u16 + 2)])
        .split(area);
    f.render_widget(Paragraph::new(left), cols[0]);
    f.render_widget(
        Paragraph::new(WATERMARK).alignment(Alignment::Right).style(
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ),
        cols[1],
    );
}
