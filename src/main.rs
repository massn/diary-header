use chrono::{DateTime, Local, NaiveDate};
use clap::{Parser, Subcommand};
use inquire::{Confirm, DateSelect, MultiSelect, Select};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::PathBuf;
use taian::{Rokuyo, calculate_rokuyo};
use tera::{Context, Tera};
use tzf_rs::DefaultFinder;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
enum DisplayItem {
    Location,
    Coordinates,
    Timezone,
    Sunrise,
    Sunset,
    Weather,
    Precipitation,
    Temperature,
    SexagenaryCycle,
    Rokuyo,
}

impl fmt::Display for DisplayItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            DisplayItem::Location => "Location (場所)",
            DisplayItem::Coordinates => "Coordinates (緯度経度)",
            DisplayItem::Timezone => "Timezone (タイムゾーン)",
            DisplayItem::Sunrise => "Sunrise (日の出)",
            DisplayItem::Sunset => "Sunset (日の入り)",
            DisplayItem::Weather => "Weather (天気)",
            DisplayItem::Precipitation => "Precipitation (降水確率)",
            DisplayItem::Temperature => "Temperature (気温)",
            DisplayItem::SexagenaryCycle => "Sexagenary Cycle (干支)",
            DisplayItem::Rokuyo => "Rokuyo (六曜)",
        };
        write!(f, "{}", s)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    language: String,
    #[serde(default = "default_display_order")]
    display_order: Vec<DisplayItem>,
}

fn default_display_order() -> Vec<DisplayItem> {
    vec![
        DisplayItem::Location,
        DisplayItem::Coordinates,
        DisplayItem::Timezone,
        DisplayItem::Sunrise,
        DisplayItem::Sunset,
        DisplayItem::Weather,
        DisplayItem::Precipitation,
        DisplayItem::Temperature,
        DisplayItem::SexagenaryCycle,
        DisplayItem::Rokuyo,
    ]
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to config file (default: ~/.config/diary-header/config.toml)
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
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
        80..=82 => {
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

fn get_rokuyo(date: NaiveDate, lang: &str) -> String {
    let rokuyo = calculate_rokuyo(date);

    match rokuyo {
        Some(rokuyo) => {
            if lang == "en" {
                match rokuyo {
                    Rokuyo::Sensho => "Sensho (先勝)".to_string(),
                    Rokuyo::Tomobiki => "Tomobiki (友引)".to_string(),
                    Rokuyo::Senbu => "Senbu (先負)".to_string(),
                    Rokuyo::Butsumetsu => "Butsumetsu (仏滅)".to_string(),
                    Rokuyo::Taian => "Taian (大安)".to_string(),
                    Rokuyo::Shakku => "Shakku (赤口)".to_string(),
                }
            } else {
                rokuyo.to_string()
            }
        }
        None => {
            if lang == "en" {
                "Unknown".to_string()
            } else {
                "不明".to_string()
            }
        }
    }
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

    // Allow user to select which items to display
    let all_items = vec![
        DisplayItem::Location,
        DisplayItem::Coordinates,
        DisplayItem::Timezone,
        DisplayItem::Sunrise,
        DisplayItem::Sunset,
        DisplayItem::Weather,
        DisplayItem::Precipitation,
        DisplayItem::Temperature,
        DisplayItem::SexagenaryCycle,
        DisplayItem::Rokuyo,
    ];

    let selected_items = MultiSelect::new(
        "Select items to display (Space to toggle, Enter to confirm):",
        all_items.clone(),
    )
    .with_default(&(0..all_items.len()).collect::<Vec<_>>())
    .prompt()?;

    // Let user specify the order by selecting items one by one
    let mut display_order: Vec<DisplayItem> = Vec::new();
    let mut remaining_items = selected_items.clone();

    while !remaining_items.is_empty() {
        let position = display_order.len() + 1;
        let prompt_text = format!(
            "Select item for position #{} (or press Esc to use default order):",
            position
        );

        let selection = Select::new(&prompt_text, remaining_items.clone()).prompt();

        match selection {
            Ok(item) => {
                display_order.push(item.clone());
                remaining_items.retain(|i| i != &item);
            }
            Err(_) => {
                // User cancelled, use default order for remaining items
                display_order.extend(remaining_items);
                break;
            }
        }
    }

    Ok(Config {
        language,
        display_order,
    })
}

fn get_config_path(custom_path: Option<PathBuf>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(path) = custom_path {
        // If custom path is provided, ensure its directory exists
        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent)?;
        }
        return Ok(path);
    }

    // Default to ~/.config/diary-header/config.toml
    let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
    let config_dir = home_dir.join(".config").join("diary-header");

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }

    Ok(config_dir.join("config.toml"))
}

fn handle_config_command(config_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("Updating configuration...");
    let new_config = prompt_for_config()?;
    let toml_string = toml::to_string(&new_config)?;
    fs::write(config_path, toml_string)?;
    println!(
        "Configuration updated and saved to: {}",
        config_path.display()
    );
    Ok(())
}

fn load_or_create_config(config_path: &PathBuf) -> Result<Config, Box<dyn std::error::Error>> {
    if config_path.exists() {
        let config_contents = fs::read_to_string(config_path)?;
        return Ok(toml::from_str(&config_contents)?);
    }

    println!("Configuration file not found at: {}", config_path.display());
    let should_create = Confirm::new("Would you like to create a new configuration file?")
        .with_default(true)
        .prompt()?;

    if !should_create {
        return Err("Configuration file is required to run this program.".into());
    }

    let new_config = prompt_for_config()?;
    let toml_string = toml::to_string(&new_config)?;
    fs::write(config_path, toml_string)?;
    println!("Configuration saved to: {}", config_path.display());

    Ok(new_config)
}

fn select_date() -> Result<NaiveDate, Box<dyn std::error::Error>> {
    let now = Local::now();
    let today = DateSelect::new("Select date for diary header:")
        .with_default(now.date_naive())
        .prompt()?;
    Ok(today)
}

fn build_location_choices() -> Vec<LocationChoice> {
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

    locations
}

fn select_location() -> Result<LocationChoice, Box<dyn std::error::Error>> {
    let locations = build_location_choices();
    let selected_loc = Select::new("Select location:", locations).prompt()?;
    Ok(selected_loc)
}

fn get_geo_info_from_selection(
    selected_loc: &LocationChoice,
) -> Result<GeoInfo, Box<dyn std::error::Error>> {
    if selected_loc.is_auto {
        return fetch_geo_info().or_else(|_| {
            Ok(GeoInfo {
                status: "fail".to_string(),
                city: "Unknown".to_string(),
                region_name: "Unknown".to_string(),
                lat: 0.0,
                lon: 0.0,
                timezone: "UTC".to_string(),
            })
        });
    }

    let finder = DefaultFinder::new();
    let tz_name = finder.get_tz_name(selected_loc.lon, selected_loc.lat);

    let parts: Vec<&str> = selected_loc.name.split(", ").collect();
    let city = parts
        .first()
        .unwrap_or(&selected_loc.name.as_str())
        .to_string();

    Ok(GeoInfo {
        status: "success".to_string(),
        city,
        region_name: selected_loc.name.clone(),
        lat: selected_loc.lat,
        lon: selected_loc.lon,
        timezone: tz_name.to_string(),
    })
}

struct DiaryData {
    geo: GeoInfo,
    sunrise_dt: DateTime<Local>,
    sunset_dt: DateTime<Local>,
    temp_max: f64,
    temp_min: f64,
    weather_code: i32,
    precip_prob: i32,
    eto: String,
    rokuyo: String,
}

fn collect_diary_data(
    geo: GeoInfo,
    today: NaiveDate,
    config: &Config,
) -> Result<DiaryData, Box<dyn std::error::Error>> {
    let coord = sunrise::Coordinates::new(geo.lat, geo.lon).ok_or("Invalid coordinates")?;
    let solar_day = sunrise::SolarDay::new(coord, today);
    let sunrise_dt: DateTime<Local> = solar_day
        .event_time(sunrise::SolarEvent::Sunrise)
        .with_timezone(&Local);
    let sunset_dt: DateTime<Local> = solar_day
        .event_time(sunrise::SolarEvent::Sunset)
        .with_timezone(&Local);

    let (temp_max, temp_min, weather_code, precip_prob) =
        fetch_weather_info(geo.lat, geo.lon, today).unwrap_or((0.0, 0.0, 0, 0));

    let eto = get_sexagenary_cycle(today);
    let rokuyo = get_rokuyo(today, &config.language);

    Ok(DiaryData {
        geo,
        sunrise_dt,
        sunset_dt,
        temp_max,
        temp_min,
        weather_code,
        precip_prob,
        eto,
        rokuyo,
    })
}

fn format_date_string(date: NaiveDate, lang: &str) -> String {
    use chrono::Datelike;
    if lang == "en" {
        date.format("%Y-%m-%d (%A)").to_string()
    } else {
        let wd_jp = match date.weekday() {
            chrono::Weekday::Mon => "月",
            chrono::Weekday::Tue => "火",
            chrono::Weekday::Wed => "水",
            chrono::Weekday::Thu => "木",
            chrono::Weekday::Fri => "金",
            chrono::Weekday::Sat => "土",
            chrono::Weekday::Sun => "日",
        };
        format!("{} ({})", date.format("%Y-%m-%d"), wd_jp)
    }
}

fn get_template_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
    let template_dir = home_dir
        .join(".config")
        .join("diary-header")
        .join("templates");

    if !template_dir.exists() {
        fs::create_dir_all(&template_dir)?;

        // Copy default templates
        let ja_template = include_str!("../templates/ja.tera");
        let en_template = include_str!("../templates/en.tera");

        fs::write(template_dir.join("ja.tera"), ja_template)?;
        fs::write(template_dir.join("en.tera"), en_template)?;
    }

    Ok(template_dir)
}

fn generate_header(
    today: NaiveDate,
    data: &DiaryData,
    config: &Config,
) -> Result<String, Box<dyn std::error::Error>> {
    let template_dir = get_template_dir()?;
    let template_path = template_dir.join(format!("{}.tera", config.language));

    if !template_path.exists() {
        return Err(format!("Template file not found: {}", template_path.display()).into());
    }

    let mut tera = Tera::default();
    tera.add_template_file(&template_path, Some(&config.language))?;

    let date_str = format_date_string(today, &config.language);
    let time_suffix = if data.geo.timezone == "Asia/Tokyo" {
        "JST"
    } else {
        ""
    };
    let time_format = format!("%H:%M:%S {}%z", time_suffix);
    let weather_desc = get_weather_description(data.weather_code, &config.language);

    let mut context = Context::new();
    context.insert("date_str", &date_str);
    context.insert("city", &data.geo.city);
    context.insert("region_name", &data.geo.region_name);
    context.insert("lat", &format!("{:.4}", data.geo.lat));
    context.insert("lon", &format!("{:.4}", data.geo.lon));
    context.insert("timezone", &data.geo.timezone);
    context.insert("sunrise", &data.sunrise_dt.format(&time_format).to_string());
    context.insert("sunset", &data.sunset_dt.format(&time_format).to_string());
    context.insert("weather", &weather_desc);
    context.insert("precipitation", &data.precip_prob);
    context.insert("temp_max", &format!("{:.1}", data.temp_max));
    context.insert("temp_min", &format!("{:.1}", data.temp_min));
    context.insert("eto", &data.eto);
    context.insert("rokuyo", &data.rokuyo);

    // Convert display_order to string names for template
    let display_order_names: Vec<String> = config
        .display_order
        .iter()
        .map(|item| format!("{:?}", item))
        .collect();
    context.insert("display_order", &display_order_names);

    let rendered = tera.render(&config.language, &context)?;
    Ok(rendered)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let config_path = get_config_path(cli.config.clone())?;

    if let Some(Commands::Config) = &cli.command {
        return handle_config_command(&config_path);
    }

    let config = load_or_create_config(&config_path)?;
    let today = select_date()?;
    let selected_loc = select_location()?;
    let geo = get_geo_info_from_selection(&selected_loc)?;
    let data = collect_diary_data(geo, today, &config)?;
    let header = generate_header(today, &data, &config)?;

    println!("{}", header);

    Ok(())
}
