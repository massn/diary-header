use chrono::{DateTime, Local, NaiveDate};
use clap::{Parser, Subcommand};
use inquire::{DateSelect, Select};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::PathBuf;
use tzf_rs::DefaultFinder;

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    language: String,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Update configuration
    Config,
}

#[derive(Deserialize, Debug)]
struct GeoInfo {
    #[allow(dead_code)]
    status: String,
    city: String,
    #[serde(rename = "regionName")]
    region_name: String,
    lat: f64,
    lon: f64,
    timezone: String,
}

#[derive(Deserialize, Debug)]
struct WeatherDaily {
    temperature_2m_max: Vec<f64>,
    temperature_2m_min: Vec<f64>,
    weather_code: Vec<i32>,
    precipitation_probability_max: Vec<i32>,
}

#[derive(Deserialize, Debug)]
struct WeatherResponse {
    daily: WeatherDaily,
}

const IP_API_URL: &str = "http://ip-api.com/json";

fn fetch_geo_info() -> Result<GeoInfo, Box<dyn std::error::Error>> {
    let resp = reqwest::blocking::get(IP_API_URL)?.json::<GeoInfo>()?;
    Ok(resp)
}

fn get_weather_description(code: i32, lang: &str) -> String {
    match code {
        0 => {
            if lang == "en" {
                "Clear sky".to_string()
            } else {
                "快晴".to_string()
            }
        }
        1 => {
            if lang == "en" {
                "Mainly clear".to_string()
            } else {
                "晴れ".to_string()
            }
        }
        2 => {
            if lang == "en" {
                "Partly cloudy".to_string()
            } else {
                "一部曇り".to_string()
            }
        }
        3 => {
            if lang == "en" {
                "Overcast".to_string()
            } else {
                "曇り".to_string()
            }
        }
        45 | 48 => {
            if lang == "en" {
                "Fog".to_string()
            } else {
                "霧".to_string()
            }
        }
        51 | 53 | 55 => {
            if lang == "en" {
                "Drizzle".to_string()
            } else {
                "霧雨".to_string()
            }
        }
        61 | 63 | 65 => {
            if lang == "en" {
                "Rain".to_string()
            } else {
                "雨".to_string()
            }
        }
        71 | 73 | 75 => {
            if lang == "en" {
                "Snow".to_string()
            } else {
                "雪".to_string()
            }
        }
        77 => {
            if lang == "en" {
                "Snow grains".to_string()
            } else {
                "雪あられ".to_string()
            }
        }
        80 | 81 | 82 => {
            if lang == "en" {
                "Rain showers".to_string()
            } else {
                "にわか雨".to_string()
            }
        }
        85 | 86 => {
            if lang == "en" {
                "Snow showers".to_string()
            } else {
                "にわか雪".to_string()
            }
        }
        95 | 96 | 99 => {
            if lang == "en" {
                "Thunderstorm".to_string()
            } else {
                "雷雨".to_string()
            }
        }
        _ => {
            if lang == "en" {
                "Unknown".to_string()
            } else {
                "不明".to_string()
            }
        }
    }
}

fn fetch_weather_info(
    lat: f64,
    lon: f64,
    date: NaiveDate,
) -> Result<(f64, f64, i32, i32), Box<dyn std::error::Error>> {
    let date_str = date.format("%Y-%m-%d").to_string();
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&daily=temperature_2m_max,temperature_2m_min,weather_code,precipitation_probability_max&start_date={}&end_date={}&timezone=auto",
        lat, lon, date_str, date_str
    );

    let resp = reqwest::blocking::get(&url)?.json::<WeatherResponse>()?;

    let temp_max = resp
        .daily
        .temperature_2m_max
        .first()
        .copied()
        .unwrap_or(0.0);
    let temp_min = resp
        .daily
        .temperature_2m_min
        .first()
        .copied()
        .unwrap_or(0.0);
    let weather_code = resp.daily.weather_code.first().copied().unwrap_or(0);
    let precip_prob = resp
        .daily
        .precipitation_probability_max
        .first()
        .copied()
        .unwrap_or(0);

    Ok((temp_max, temp_min, weather_code, precip_prob))
}

fn get_sexagenary_cycle(date: NaiveDate) -> String {
    let stems = ["甲", "乙", "丙", "丁", "戊", "己", "庚", "辛", "壬", "癸"];
    let branches = [
        "子", "丑", "寅", "卯", "辰", "巳", "午", "未", "申", "酉", "戌", "亥",
    ];

    let anchor = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let diff = date.signed_duration_since(anchor).num_days();

    // Handle negative diff if needed, but for future dates it's positive.
    // (diff % 10 + 10) % 10 ensures positive index.
    let stem_idx = ((diff % 10 + 10) % 10) as usize;
    let branch_idx = ((diff % 12 + 12) % 12) as usize;

    format!("{}{}", stems[stem_idx], branches[branch_idx])
}

#[derive(Clone)]
struct LocationChoice {
    name: String,
    is_auto: bool,
    lat: f64,
    lon: f64,
}

impl fmt::Display for LocationChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

fn prompt_for_config() -> Result<Config, Box<dyn std::error::Error>> {
    let language_opt = Select::new(
        "Select language for the diary header:",
        vec!["ja (Japanese)", "en (English)"],
    )
    .prompt()?;

    let language = if language_opt.starts_with("en") {
        "en".to_string()
    } else {
        "ja".to_string()
    };

    Ok(Config { language })
}

fn get_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let config_dir = dirs::config_dir()
        .ok_or("Could not find config directory")?
        .join("diary-header");

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }

    Ok(config_dir.join("config.toml"))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let config_path = get_config_path()?;

    match &cli.command {
        Some(Commands::Config) => {
            println!("Updating configuration...");
            let new_config = prompt_for_config()?;
            let toml_string = toml::to_string(&new_config)?;
            fs::write(&config_path, toml_string)?;
            println!(
                "Configuration updated and saved to: {}",
                config_path.display()
            );
            return Ok(());
        }
        None => {}
    }

    // 2. Check if TOML config file exists
    let _config = if config_path.exists() {
        let config_contents = fs::read_to_string(&config_path)?;
        toml::from_str(&config_contents)?
    } else {
        println!("Configuration file not found. Starting initial setup.");
        // 3. Prompt interactive setup
        let new_config = prompt_for_config()?;

        let toml_string = toml::to_string(&new_config)?;
        fs::write(&config_path, toml_string)?;
        println!("Configuration saved to: {}", config_path.display());

        new_config
    };

    let now = Local::now();

    let today = DateSelect::new("Select date for diary header:")
        .with_default(now.date_naive())
        .prompt()?;

    let mut locations = vec![LocationChoice {
        name: "Current Location (Auto via IP)".to_string(),
        is_auto: true,
        lat: 0.0,
        lon: 0.0,
    }];

    for city in cities::all() {
        locations.push(LocationChoice {
            name: format!("{}, {}", city.city, city.country),
            is_auto: false,
            lat: city.latitude,
            lon: city.longitude,
        });
    }

    let selected_loc = Select::new("Select location:", locations).prompt()?;

    // Fetch Geo Info
    let geo = if selected_loc.is_auto {
        fetch_geo_info().unwrap_or(GeoInfo {
            status: "fail".to_string(),
            city: "Unknown".to_string(),
            region_name: "Unknown".to_string(),
            lat: 0.0,
            lon: 0.0,
            timezone: "UTC".to_string(),
        })
    } else {
        let finder = DefaultFinder::new();
        let tz_name = finder.get_tz_name(selected_loc.lon, selected_loc.lat);

        let parts: Vec<&str> = selected_loc.name.split(", ").collect();
        let city = parts
            .first()
            .unwrap_or(&selected_loc.name.as_str())
            .to_string();

        GeoInfo {
            status: "success".to_string(),
            city,
            region_name: selected_loc.name.clone(),
            lat: selected_loc.lat,
            lon: selected_loc.lon,
            timezone: tz_name.to_string(),
        }
    };

    // Calculate Sunrise/Sunset
    let coord = sunrise::Coordinates::new(geo.lat, geo.lon).unwrap();
    let solar_day = sunrise::SolarDay::new(coord, today);
    let sunrise_dt: DateTime<Local> = solar_day
        .event_time(sunrise::SolarEvent::Sunrise)
        .with_timezone(&Local);
    let sunset_dt: DateTime<Local> = solar_day
        .event_time(sunrise::SolarEvent::Sunset)
        .with_timezone(&Local);

    // Fetch Weather Information
    let (temp_max, temp_min, weather_code, precip_prob) =
        fetch_weather_info(geo.lat, geo.lon, today).unwrap_or((0.0, 0.0, 0, 0));

    // Calculate Sexagenary Cycle
    let eto = get_sexagenary_cycle(today);

    use chrono::Datelike;
    let date_str = if _config.language == "en" {
        today.format("%Y-%m-%d (%A)").to_string()
    } else {
        let wd_jp = match today.weekday() {
            chrono::Weekday::Mon => "月",
            chrono::Weekday::Tue => "火",
            chrono::Weekday::Wed => "水",
            chrono::Weekday::Thu => "木",
            chrono::Weekday::Fri => "金",
            chrono::Weekday::Sat => "土",
            chrono::Weekday::Sun => "日",
        };
        format!("{} ({})", today.format("%Y-%m-%d"), wd_jp)
    };

    let time_suffix = if geo.timezone == "Asia/Tokyo" {
        "JST"
    } else {
        ""
    };
    let time_format = format!("%H:%M:%S {}%z", time_suffix);

    let weather_desc = get_weather_description(weather_code, &_config.language);

    let header = if _config.language == "en" {
        format!(
            "## {}\n\
             - Location (Current IP Address): {} ({})\n\
             - Lat/Lon: ({:.4}, {:.4})\n\
             - Timezone: {}\n\
             - Sunrise: {}\n\
             - Sunset: {}\n\
             - Weather: {}\n\
             - Precipitation Probability: {}%\n\
             - Temperature: Max {:.1}°C / Min {:.1}°C\n\
             - Sexagenary Cycle: {}",
            date_str,
            geo.city,
            geo.region_name,
            geo.lat,
            geo.lon,
            geo.timezone,
            sunrise_dt.format(&time_format),
            sunset_dt.format(&time_format),
            weather_desc,
            precip_prob,
            temp_max,
            temp_min,
            eto
        )
    } else {
        format!(
            "## {}\n\
             - 場所 (Current IP Address): {} ({})\n\
             - 緯度経度: ({:.4}, {:.4})\n\
             - タイムゾーン: {}\n\
             - 日の出: {}\n\
             - 日の入り: {}\n\
             - 天気: {}\n\
             - 降水確率: {}%\n\
             - 気温: 最高 {:.1}°C / 最低 {:.1}°C\n\
             - 干支: {}",
            date_str,
            geo.city,
            geo.region_name,
            geo.lat,
            geo.lon,
            geo.timezone,
            sunrise_dt.format(&time_format),
            sunset_dt.format(&time_format),
            weather_desc,
            precip_prob,
            temp_max,
            temp_min,
            eto
        )
    };

    println!("{}", header);

    Ok(())
}
