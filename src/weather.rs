//! Weather API client + helpers (free no-key JSON endpoints: forecast + city search).

use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde::Deserialize;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Units
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Units {
    Metric,
    Imperial,
}

impl Units {
    pub fn toggle(self) -> Self {
        match self {
            Units::Metric => Units::Imperial,
            Units::Imperial => Units::Metric,
        }
    }

    pub fn temp_param(self) -> &'static str {
        match self {
            Units::Metric => "celsius",
            Units::Imperial => "fahrenheit",
        }
    }

    pub fn wind_param(self) -> &'static str {
        match self {
            Units::Metric => "kmh",
            Units::Imperial => "mph",
        }
    }

    pub fn temp_label(self) -> &'static str {
        match self {
            Units::Metric => "°C",
            Units::Imperial => "°F",
        }
    }

    pub fn wind_label(self) -> &'static str {
        match self {
            Units::Metric => "km/h",
            Units::Imperial => "mph",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Units::Metric => "Metric",
            Units::Imperial => "Imperial",
        }
    }
}

// ---------------------------------------------------------------------------
// Places
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct City {
    pub name: String,
    pub country: String,
    pub lat: f64,
    pub lon: f64,
}

impl City {
    pub fn label(&self) -> String {
        if self.country.is_empty() {
            self.name.clone()
        } else {
            format!("{}, {}", self.name, self.country)
        }
    }

    /// Stable key used for the favorites file.
    pub fn key(&self) -> String {
        format!("{}|{}|{}|{}", self.name, self.country, self.lat, self.lon)
    }
}

pub fn preset_cities() -> Vec<City> {
    vec![
        City { name: "Bengaluru".into(), country: "India".into(), lat: 12.9716, lon: 77.5946 },
        City { name: "London".into(), country: "UK".into(), lat: 51.5074, lon: -0.1278 },
        City { name: "New York".into(), country: "USA".into(), lat: 40.7128, lon: -74.0060 },
        City { name: "Tokyo".into(), country: "Japan".into(), lat: 35.6762, lon: 139.6503 },
        City { name: "Nairobi".into(), country: "Kenya".into(), lat: -1.2921, lon: 36.8219 },
        City { name: "Sydney".into(), country: "Australia".into(), lat: -33.8688, lon: 151.2093 },
        City { name: "Rio de Janeiro".into(), country: "Brazil".into(), lat: -22.9068, lon: -43.1729 },
    ]
}

// ---------------------------------------------------------------------------
// WMO weather codes + compass
// ---------------------------------------------------------------------------

/// Returns (icon, description) for a WMO weather code.
pub fn wmo_info(code: u8) -> (&'static str, &'static str) {
    match code {
        0 => ("☀", "Clear sky"),
        1 => ("🌤", "Mainly clear"),
        2 => ("⛅", "Partly cloudy"),
        3 => ("☁", "Overcast"),
        45 => ("🌫", "Fog"),
        48 => ("🌫", "Rime fog"),
        51 => ("🌦", "Light drizzle"),
        53 => ("🌦", "Drizzle"),
        55 => ("🌦", "Dense drizzle"),
        56 | 57 => ("🌧", "Freezing drizzle"),
        61 => ("🌧", "Slight rain"),
        63 => ("🌧", "Rain"),
        65 => ("🌧", "Heavy rain"),
        66 | 67 => ("🌧", "Freezing rain"),
        71 => ("❄", "Slight snow"),
        73 => ("❄", "Snow"),
        75 => ("❄", "Heavy snow"),
        77 => ("❄", "Snow grains"),
        80 => ("🌧", "Slight showers"),
        81 => ("🌧", "Showers"),
        82 => ("🌧", "Violent showers"),
        95 => ("⛈", "Thunderstorm"),
        96 | 99 => ("⛈", "Storm + hail"),
        _ => ("🌡", "Unknown"),
    }
}

/// Degrees (meteorological: direction wind comes FROM) → 16-point compass label.
pub fn compass(deg: f64) -> &'static str {
    const PTS: [&str; 16] = [
        "N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE",
        "S", "SSW", "SW", "WSW", "W", "WNW", "NW", "NNW",
    ];
    let d = ((deg % 360.0) + 360.0) % 360.0;
    PTS[((d + 11.25) / 22.5) as usize % 16]
}

// ---------------------------------------------------------------------------
// Parsed weather data (what the UI renders)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Current {
    pub temp: f64,
    pub feels: f64,
    pub humidity: f64,
    pub code: u8,
    pub wind_speed: f64,
    pub wind_dir: f64,
    pub pressure: f64,
    pub time: String,
}

#[derive(Debug, Clone)]
pub struct Hour {
    pub time: String,
    pub temp: f64,
    pub precip: f64,
    pub code: u8,
}

#[derive(Debug, Clone)]
pub struct Day {
    pub label: String,
    pub code: u8,
    pub max: f64,
    pub min: f64,
    pub precip_max: f64,
    pub sunrise: String,
    pub sunset: String,
}

#[derive(Debug, Clone)]
pub struct WeatherData {
    pub city: City,
    pub units: Units,
    pub timezone: String,
    pub current: Current,
    pub hourly: Vec<Hour>,
    pub daily: Vec<Day>,
}

// ---------------------------------------------------------------------------
// Raw API shapes (defensive: everything Option)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Default)]
struct ForecastResp {
    timezone: Option<String>,
    current: Option<CurrentResp>,
    hourly: Option<HourlyResp>,
    daily: Option<DailyResp>,
}

#[derive(Debug, Deserialize, Default)]
struct CurrentResp {
    temperature_2m: Option<f64>,
    relative_humidity_2m: Option<f64>,
    apparent_temperature: Option<f64>,
    weather_code: Option<u8>,
    wind_speed_10m: Option<f64>,
    wind_direction_10m: Option<f64>,
    pressure_msl: Option<f64>,
    time: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct HourlyResp {
    time: Option<Vec<String>>,
    temperature_2m: Option<Vec<Option<f64>>>,
    precipitation_probability: Option<Vec<Option<f64>>>,
    weather_code: Option<Vec<Option<u8>>>,
}

#[derive(Debug, Deserialize, Default)]
struct DailyResp {
    time: Option<Vec<String>>,
    weather_code: Option<Vec<Option<u8>>>,
    temperature_2m_max: Option<Vec<Option<f64>>>,
    temperature_2m_min: Option<Vec<Option<f64>>>,
    precipitation_probability_max: Option<Vec<Option<f64>>>,
    sunrise: Option<Vec<String>>,
    sunset: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct GeocodeResp {
    results: Option<Vec<GeocodeHit>>,
}

#[derive(Debug, Deserialize)]
struct GeocodeHit {
    name: Option<String>,
    country: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
}

// ---------------------------------------------------------------------------
// Small format helpers
// ---------------------------------------------------------------------------

fn hhmm(iso: &str) -> String {
    iso.split('T').last().unwrap_or(iso).chars().take(5).collect()
}

fn day_label(iso_date: &str) -> String {
    match NaiveDate::parse_from_str(iso_date, "%Y-%m-%d") {
        Ok(d) => d.format("%a %m/%d").to_string(),
        Err(_) => iso_date.to_string(),
    }
}

fn client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("srk-weather-tui/0.1")
        .build()
        .context("building HTTP client")
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn fetch_forecast(city: &City, units: Units) -> Result<WeatherData> {
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}\
         &current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,\
         wind_speed_10m,wind_direction_10m,pressure_msl\
         &hourly=temperature_2m,precipitation_probability,weather_code\
         &daily=weather_code,temperature_2m_max,temperature_2m_min,\
         precipitation_probability_max,sunrise,sunset\
         &timezone=auto&forecast_days=7&temperature_unit={}&wind_speed_unit={}",
        city.lat,
        city.lon,
        units.temp_param(),
        units.wind_param()
    );

    let resp: ForecastResp = client()?
        .get(&url)
        .send()
        .context("requesting forecast")?
        .error_for_status()
        .context("forecast API returned an error status")?
        .json()
        .context("parsing forecast JSON")?;

    let c = resp.current.unwrap_or_default();
    let current = Current {
        temp: c.temperature_2m.unwrap_or(f64::NAN),
        feels: c.apparent_temperature.unwrap_or(f64::NAN),
        humidity: c.relative_humidity_2m.unwrap_or(f64::NAN),
        code: c.weather_code.unwrap_or(255),
        wind_speed: c.wind_speed_10m.unwrap_or(f64::NAN),
        wind_dir: c.wind_direction_10m.unwrap_or(f64::NAN),
        pressure: c.pressure_msl.unwrap_or(f64::NAN),
        time: c.time.map(|t| hhmm(&t)).unwrap_or_else(|| "--:--".into()),
    };

    let mut hourly = Vec::new();
    if let Some(h) = resp.hourly {
        let times = h.time.unwrap_or_default();
        let temps = h.temperature_2m.unwrap_or_default();
        let precs = h.precipitation_probability.unwrap_or_default();
        let codes = h.weather_code.unwrap_or_default();
        // Find "now" in the hourly series; fall back to index 0.
        let now_key = chrono::Local::now().format("%Y-%m-%dT%H").to_string();
        let start = times.iter().position(|t| t.starts_with(&now_key)).unwrap_or(0);
        for i in start..(start + 24).min(times.len()) {
            hourly.push(Hour {
                time: hhmm(times.get(i).map(String::as_str).unwrap_or("--")),
                temp: temps.get(i).and_then(|v| *v).unwrap_or(f64::NAN),
                precip: precs.get(i).and_then(|v| *v).unwrap_or(0.0),
                code: codes.get(i).and_then(|v| *v).unwrap_or(255),
            });
        }
    }

    let mut daily = Vec::new();
    if let Some(d) = resp.daily {
        let dates = d.time.unwrap_or_default();
        let codes = d.weather_code.unwrap_or_default();
        let maxs = d.temperature_2m_max.unwrap_or_default();
        let mins = d.temperature_2m_min.unwrap_or_default();
        let precs = d.precipitation_probability_max.unwrap_or_default();
        let rises = d.sunrise.unwrap_or_default();
        let sets = d.sunset.unwrap_or_default();
        for i in 0..dates.len().min(7) {
            daily.push(Day {
                label: day_label(&dates[i]),
                code: codes.get(i).and_then(|v| *v).unwrap_or(255),
                max: maxs.get(i).and_then(|v| *v).unwrap_or(f64::NAN),
                min: mins.get(i).and_then(|v| *v).unwrap_or(f64::NAN),
                precip_max: precs.get(i).and_then(|v| *v).unwrap_or(0.0),
                sunrise: rises.get(i).map(|s| hhmm(s)).unwrap_or_else(|| "--:--".into()),
                sunset: sets.get(i).map(|s| hhmm(s)).unwrap_or_else(|| "--:--".into()),
            });
        }
    }

    Ok(WeatherData {
        city: city.clone(),
        units,
        timezone: resp.timezone.unwrap_or_else(|| "UTC".into()),
        current,
        hourly,
        daily,
    })
}

pub fn geocode(query: &str) -> Result<Vec<City>> {    let url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=5&language=en&format=json",
        query
    );
    let resp: GeocodeResp = client()?
        .get(&url)
        .send()
        .context("requesting city search")?
        .error_for_status()
        .context("geocoding API returned an error status")?
        .json()
        .context("parsing geocoding JSON")?;

    Ok(resp
        .results
        .unwrap_or_default()
        .into_iter()
        .filter_map(|h| {
            Some(City {
                name: h.name?,
                country: h.country.unwrap_or_default(),
                lat: h.latitude?,
                lon: h.longitude?,
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wmo_codes_map() {
        assert_eq!(wmo_info(0).1, "Clear sky");
        assert_eq!(wmo_info(95).1, "Thunderstorm");
        assert_eq!(wmo_info(71).1, "Slight snow");
    }

    #[test]
    fn compass_points() {
        assert_eq!(compass(0.0), "N");
        assert_eq!(compass(90.0), "E");
        assert_eq!(compass(180.0), "S");
        assert_eq!(compass(270.0), "W");
        assert_eq!(compass(45.0), "NE");
    }

    #[test]
    fn parses_sample_forecast() {
        let sample = serde_json::json!({
            "timezone": "Europe/London",
            "current": {
                "temperature_2m": 18.6, "relative_humidity_2m": 70.0,
                "apparent_temperature": 17.0, "weather_code": 2,
                "wind_speed_10m": 12.5, "wind_direction_10m": 200.0,
                "pressure_msl": 1013.0, "time": "2026-10-09T17:30"
            },
            "hourly": {
                "time": ["2026-10-09T17:00"],
                "temperature_2m": [18.6],
                "precipitation_probability": [10.0],
                "weather_code": [2]
            },
            "daily": {
                "time": ["2026-10-09"],
                "weather_code": [2],
                "temperature_2m_max": [19.0],
                "temperature_2m_min": [12.0],
                "precipitation_probability_max": [20.0],
                "sunrise": ["2026-10-09T06:50"],
                "sunset": ["2026-10-09T18:40"]
            }
        });
        let resp: ForecastResp = serde_json::from_value(sample).unwrap();
        assert_eq!(resp.timezone.as_deref(), Some("Europe/London"));
        assert_eq!(resp.current.unwrap().weather_code, Some(2));
        let d = resp.daily.unwrap();
        assert_eq!(day_label(&d.time.unwrap()[0]), "Fri 10/09");
    }

    #[test]
    fn parses_sample_geocode() {
        let sample = serde_json::json!({
            "results": [{"name": "Paris", "country": "France",
                         "latitude": 48.85, "longitude": 2.35}]
        });
        let resp: GeocodeResp = serde_json::from_value(sample).unwrap();
        let r = &resp.results.unwrap()[0];
        assert_eq!(r.name.as_deref(), Some("Paris"));
        assert_eq!(r.latitude, Some(48.85));
    }
}
