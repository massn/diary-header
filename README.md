# diary-header

A command-line tool written in Rust that interactively generates a Markdown header for your daily diary entries. It automatically retrieves geolocation data, calculates sunrise/sunset times, and determines the sexagenary cycle (干支 - Eto) for any given date.

## Features

- **Interactive Prompts**: Easily select the date and location using an interactive terminal UI.
- **Location Auto-Detection**: Automatically fetches your current location, latitude, longitude, and timezone based on your IP address.
- **Manual Location Selection**: Optionally select from a comprehensive list of cities worldwide if you are writing a diary for somewhere else.
- **Astronomical Calculations**: Calculates precise sunrise and sunset times based on the selected date and location coordinates.
- **Weather Information**: Retrieves daily maximum and minimum temperatures for the selected date and location.
- **Sexagenary Cycle (干支)**: Automatically computes the traditional continuous calendar cycle (Eto) for the given date.
- **Six-Day Cycle (六曜)**: Calculates the traditional Japanese six-day cycle (Rokuyo) for fortune-telling and event planning.
- **Language Configuration**: Choose between Japanese (ja) and English (en) for the output format.
- **Customizable Display Order**: Configure which information to display and in what order via the config file.
- **Persistent Configuration**: Prompts for an initial configuration (language) and saves it to a configuration file (`~/.config/diary-header/config.toml`). You can update this later using the `config` subcommand.

## Installation

### Via npm (Recommended)

The easiest way to install `diary-header` is via npm:

```bash
npm install -g diary-header
```

This will download a prebuilt binary for your platform and make the `diary-header` command available globally.

### Via Cargo

Alternatively, if you have Rust and Cargo installed, you can install from source:

```bash
git clone https://github.com/massn/diary-header.git
cd diary-header
cargo install --path .
```

Or just run it directly:

```bash
cargo run
```

## Usage

When you run `diary-header` (or `cargo run`), the tool will interactively prompt you for the necessary information:

1. **First-time Setup**: If the configuration file does not exist, it will ask for your preferred output language (Japanese or English). This is saved to `~/.config/diary-header/config.toml`.
2. **Date Selection**: Prompts you to select a date for the diary header (defaults to today).
3. **Location Selection**: Prompts you to choose "Current Location (Auto via IP)" or select a specific city from the list.

### Example Output

Once the selections are made, the tool outputs a localized Markdown header depending on your configured language:

**Japanese Output (`ja`):**
```markdown
## 2024-11-20 (水)
- 場所 (Current IP Address): Tokyo (Tokyo)
- 緯度経度: (35.6895, 139.6917)
- タイムゾーン: Asia/Tokyo
- 日の出: 06:21:43 JST+0900
- 日の入り: 16:32:11 JST+0900
- 天気: 晴れ
- 降水確率: 20%
- 気温: 最高 18.5°C / 最低 12.3°C
- 干支: 甲辰
- 六曜: 先勝
```

**English Output (`en`):**
```markdown
## 2024-11-20 (Wednesday)
- Location (Current IP Address): Tokyo (Tokyo)
- Lat/Lon: (35.6895, 139.6917)
- Timezone: Asia/Tokyo
- Sunrise: 06:21:43 JST+0900
- Sunset: 16:32:11 JST+0900
- Weather: Mainly clear
- Precipitation Probability: 20%
- Temperature: Max 18.5°C / Min 12.3°C
- Sexagenary Cycle: 甲辰
- Rokuyo: 先勝
```

You can redirect this output directly to your diary file or copy it securely:

```bash
diary-header >> my-diary.md
```

## Configuration

The configuration file is stored in `~/.config/diary-header/config.toml` (macOS/Linux) or `%APPDATA%\diary-header\config.toml` (Windows).

### Language Setting

You can interactively change the language setting at any time by using the `config` subcommand:

```bash
diary-header config
# Or with cargo:
cargo run -- config
```

### Display Order Customization

You can customize the order of displayed information by manually editing the configuration file. By default, all items are displayed in the following order:

```toml
language = "en"
display_order = [
    "location",
    "coordinates",
    "timezone",
    "sunrise",
    "sunset",
    "weather",
    "precipitation",
    "temperature",
    "sexagenary_cycle",
    "rokuyo",
]
```

**Available items:**
- `location` - Location information (city/region)
- `coordinates` - Latitude and longitude
- `timezone` - Timezone
- `sunrise` - Sunrise time
- `sunset` - Sunset time
- `weather` - Weather condition
- `precipitation` - Precipitation probability
- `temperature` - Max/min temperature
- `sexagenary_cycle` - Sexagenary cycle (干支)
- `rokuyo` - Six-day cycle (六曜)

You can reorder these items or remove items you don't need. For example, to show only weather-related information:

```toml
language = "ja"
display_order = [
    "weather",
    "temperature",
    "precipitation",
]
```

Items not included in `display_order` will not be displayed in the output.

## Dependencies

- **[inquire](https://crates.io/crates/inquire)**: For interactive terminal prompts (date, select, text).
- **[reqwest](https://crates.io/crates/reqwest)**: For hitting the `ip-api.com` endpoint to fetch IP-based timezone and geolocation info, and the Open-Meteo API for weather data.
- **[sunrise](https://crates.io/crates/sunrise)**: For calculating the exact sunrise and sunset times.
- **[tzf-rs](https://crates.io/crates/tzf-rs)**: For fast offline timezone timezone lookups based on longitude and latitude.
- **[chrono](https://crates.io/crates/chrono)**: For date and time manipulation.
- **[cities](https://crates.io/crates/cities)**: For the embedded list of global cities.
- **[clap](https://crates.io/crates/clap)**: For CLI argument parsing and subcommands.
- **[serde](https://crates.io/crates/serde) / [serde_json](https://crates.io/crates/serde_json) / [toml](https://crates.io/crates/toml)**: For data serialization and writing/reading the config.

## Development

### Release Process

The version is managed through `package.json`, which serves as the single source of truth. The `build.rs` script automatically reads the version from `package.json` during compilation.

To create a new release:

1. **Update the version in `package.json`:**
   ```bash
   # Edit package.json and update the version field
   # Example: "version": "0.1.9"
   ```

2. **Update `Cargo.toml` version (optional but recommended):**
   ```bash
   # Keep Cargo.toml in sync to avoid confusion
   # Example: version = "0.1.9"
   ```

3. **Update `Cargo.lock`:**
   ```bash
   cargo update -p diary-header
   ```

4. **Build and verify the version:**
   ```bash
   cargo build --release
   ./target/release/diary-header --version
   # Should output: diary-header 0.1.9
   ```

5. **Commit the changes:**
   ```bash
   git add package.json Cargo.toml Cargo.lock
   git commit -m "chore: bump version to 0.1.9"
   ```

6. **Create a git tag:**
   ```bash
   git tag -a v0.1.9 -m "Release v0.1.9"
   ```

7. **Push to remote:**
   ```bash
   git push origin main
   git push origin v0.1.9
   ```

8. **Publish to npm:**
   ```bash
   npm publish
   ```

**Note:** The version displayed by `--version` comes from `package.json`, not from git tags. This ensures consistent versioning across npm packages and the binary.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
