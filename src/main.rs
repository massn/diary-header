use chrono::{DateTime, Local, NaiveDate};
use inquire::{DateSelect, Select, Text};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use tzf_rs::DefaultFinder;

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    title_prefix: String,
    author: String,
    tags: String,
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

const IP_API_URL: &str = "http://ip-api.com/json";

fn fetch_geo_info() -> Result<GeoInfo, Box<dyn std::error::Error>> {
    let resp = reqwest::blocking::get(IP_API_URL)?.json::<GeoInfo>()?;
    Ok(resp)
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Locate or create config directory
    let config_dir = dirs::config_dir()
        .ok_or("Could not find config directory")?
        .join("diary-header");

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }

    let config_path = config_dir.join("config.toml");

    // 2. Check if TOML config file exists
    let _config = if config_path.exists() {
        let config_contents = fs::read_to_string(&config_path)?;
        toml::from_str(&config_contents)?
    } else {
        println!("Configuration file not found. Starting initial setup.");
        // 3. Prompt interactive setup
        let title_prefix = Text::new("Enter the prefix for the diary title (e.g., 'Diary - '):")
            .with_default("Diary - ")
            .prompt()?;
        let author = Text::new("Enter the author name:").prompt()?;
        let tags =
            Text::new("Enter default tags separated by commas (e.g., 'diary,tech'):").prompt()?;

        let new_config = Config {
            title_prefix,
            author,
            tags,
        };

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

    // Calculate Sexagenary Cycle
    let eto = get_sexagenary_cycle(today);

    // Format Header
    let date_str = today.format("%Y-%m-%d (%A)").to_string();
    let time_suffix = if geo.timezone == "Asia/Tokyo" {
        "JST"
    } else {
        ""
    };
    let time_format = format!("%H:%M:%S {}%z", time_suffix);

    let header = format!(
        "## {}\n\
             - 場所 (Current IP Address): {} ({})\n\
             - 緯度経度: ({:.4}, {:.4})\n\
             - タイムゾーン: {}\n\
             - 日の出: {}\n\
             - 日の入り: {}\n\
             - 干支: {}",
        date_str,
        geo.city,
        geo.region_name,
        geo.lat,
        geo.lon,
        geo.timezone,
        sunrise_dt.format(&time_format),
        sunset_dt.format(&time_format),
        eto
    );

    println!("{}", header);

    Ok(())
}
