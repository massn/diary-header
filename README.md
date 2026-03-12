# diary-header

A command-line tool written in Rust that interactively generates a Markdown header for your daily diary entries. It automatically retrieves geolocation data, calculates sunrise/sunset times, and determines the sexagenary cycle (干支 - Eto) for any given date.

## Features

- **Interactive Prompts**: Easily select the date and location using an interactive terminal UI.
- **Location Auto-Detection**: Automatically fetches your current location, latitude, longitude, and timezone based on your IP address.
- **Manual Location Selection**: Optionally select from a comprehensive list of cities worldwide if you are writing a diary for somewhere else.
- **Astronomical Calculations**: Calculates precise sunrise and sunset times based on the selected date and location coordinates.
- **Sexagenary Cycle (干支)**: Automatically computes the traditional continuous calendar cycle (Eto) for the given date.
- **Persistent Configuration**: Prompts for an initial configuration (title prefix, author, tags) and saves it to a configuration file (`~/.config/diary-header/config.toml`).

## Installation

Ensure you have Rust and Cargo installed. Then, clone the repository and build the project:

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

1. **First-time Setup**: If the configuration file does not exist, it will ask for a diary title prefix, author name, and default tags. This is saved to `~/.config/diary-header/config.toml`.
2. **Date Selection**: Prompts you to select a date for the diary header (defaults to today).
3. **Location Selection**: Prompts you to choose "Current Location (Auto via IP)" or select a specific city from the list.

### Example Output

Once the selections are made, the tool outputs a localized Markdown header:

```markdown
## 2024-11-20 (Wednesday)
- 場所 (Current IP Address): Tokyo (Tokyo)
- 緯度経度: (35.6895, 139.6917)
- タイムゾーン: Asia/Tokyo
- 日の出: 06:21:43 JST+0900
- 日の入り: 16:32:11 JST+0900
- 干支: 甲辰
```

You can redirect this output directly to your diary file or copy it securely:

```bash
diary-header >> my-diary.md
```

## Configuration

The configuration file is stored in `~/.config/diary-header/config.toml` (macOS/Linux) or `%APPDATA%\diary-header\config.toml` (Windows). It looks like this:

```toml
title_prefix = "Diary - "
author = "Your Name"
tags = "diary,tech"
```

## Dependencies

- **[inquire](https://crates.io/crates/inquire)**: For interactive terminal prompts (date, select, text).
- **[reqwest](https://crates.io/crates/reqwest)**: For hitting the `ip-api.com` endpoint to fetch IP-based timezone and geolocation info.
- **[sunrise](https://crates.io/crates/sunrise)**: For calculating the exact sunrise and sunset times.
- **[tzf-rs](https://crates.io/crates/tzf-rs)**: For fast offline timezone timezone lookups based on longitude and latitude.
- **[chrono](https://crates.io/crates/chrono)**: For date and time manipulation.
- **[cities](https://crates.io/crates/cities)**: For the embedded list of global cities.
- **[serde](https://crates.io/crates/serde) / [serde_json](https://crates.io/crates/serde_json) / [toml](https://crates.io/crates/toml)**: For data serialization and writing/reading the config.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
