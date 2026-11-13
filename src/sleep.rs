//! Fetches last night's sleep stage breakdown from the Google Health API (v4).
//!
//! Authentication uses the OAuth 2.0 loopback flow for installed apps.
//! Run `diary-header auth` once to obtain a refresh token, which is stored
//! next to the config file and reused on subsequent runs.

use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const SLEEP_RECONCILE_URL: &str =
    "https://health.googleapis.com/v4/users/me/dataTypes/sleep/dataPoints:reconcile";
const SLEEP_SCOPE: &str = "https://www.googleapis.com/auth/googlehealth.sleep.readonly";
const TOKEN_FILE_NAME: &str = "google_health_token.json";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GoogleHealthConfig {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct StoredToken {
    refresh_token: String,
}

#[derive(Deserialize, Debug)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
}

/// Minutes spent in each stage during one sleep session.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SleepBreakdown {
    /// Total minutes asleep (light + deep + REM, excluding awake).
    pub total: i64,
    pub deep: i64,
    pub light: i64,
    pub rem: i64,
    /// Minutes to fall asleep (sleep onset latency).
    pub to_fall_asleep: i64,
    pub awake: i64,
}

// ---- API response types -------------------------------------------------

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ReconcileResponse {
    #[serde(default)]
    data_points: Vec<DataPoint>,
    #[serde(default)]
    next_page_token: Option<String>,
}

#[derive(Deserialize, Debug)]
struct DataPoint {
    sleep: Option<Sleep>,
}

#[derive(Deserialize, Debug)]
struct Sleep {
    #[serde(default)]
    metadata: SleepMetadata,
    summary: Option<SleepSummary>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct SleepMetadata {
    #[serde(default)]
    main_sleep: bool,
    // The documentation example shows `main` instead of `mainSleep`; accept both.
    #[serde(default)]
    main: bool,
    #[serde(default)]
    nap: bool,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct SleepSummary {
    #[serde(default)]
    stages_summary: Vec<StageSummary>,
    minutes_in_sleep_period: Option<String>,
    minutes_to_fall_asleep: Option<String>,
    minutes_asleep: Option<String>,
    minutes_awake: Option<String>,
}

#[derive(Deserialize, Debug)]
struct StageSummary {
    #[serde(rename = "type")]
    stage_type: String,
    minutes: Option<String>,
}

fn parse_minutes(s: &Option<String>) -> i64 {
    s.as_deref().and_then(|v| v.parse().ok()).unwrap_or(0)
}

impl Sleep {
    fn is_main(&self) -> bool {
        self.metadata.main_sleep || self.metadata.main
    }

    fn period_minutes(&self) -> i64 {
        self.summary
            .as_ref()
            .map(|s| parse_minutes(&s.minutes_in_sleep_period))
            .unwrap_or(0)
    }

    fn breakdown(&self) -> Option<SleepBreakdown> {
        let summary = self.summary.as_ref()?;
        let mut b = SleepBreakdown::default();
        let mut awake_from_stages = None;
        for stage in &summary.stages_summary {
            let minutes = parse_minutes(&stage.minutes);
            match stage.stage_type.as_str() {
                "DEEP" => b.deep += minutes,
                "LIGHT" => b.light += minutes,
                "REM" => b.rem += minutes,
                "AWAKE" => *awake_from_stages.get_or_insert(0) += minutes,
                _ => {}
            }
        }
        b.awake = awake_from_stages.unwrap_or_else(|| parse_minutes(&summary.minutes_awake));
        b.to_fall_asleep = parse_minutes(&summary.minutes_to_fall_asleep);
        b.total = match summary.minutes_asleep {
            Some(_) => parse_minutes(&summary.minutes_asleep),
            None => b.deep + b.light + b.rem,
        };
        Some(b)
    }
}

/// Picks the main sleep session; falls back to the longest non-nap session.
fn select_main_sleep(sleeps: &[Sleep]) -> Option<&Sleep> {
    sleeps.iter().find(|s| s.is_main()).or_else(|| {
        sleeps
            .iter()
            .filter(|s| !s.metadata.nap)
            .max_by_key(|s| s.period_minutes())
    })
}

// ---- Token handling -----------------------------------------------------

pub fn token_path(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .map(|p| p.join(TOKEN_FILE_NAME))
        .unwrap_or_else(|| PathBuf::from(TOKEN_FILE_NAME))
}

fn refresh_access_token(
    cfg: &GoogleHealthConfig,
    refresh_token: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(TOKEN_URL)
        .form(&[
            ("client_id", cfg.client_id.as_str()),
            ("client_secret", cfg.client_secret.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!(
            "failed to refresh access token ({status}): {body}. Run `diary-header auth` again."
        )
        .into());
    }
    Ok(resp.json::<TokenResponse>()?.access_token)
}

fn random_state() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}{:x}", nanos, std::process::id())
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    out.push(v);
                    i += 2;
                } else {
                    out.push(b'%');
                }
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn parse_query(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter_map(|kv| {
            let (k, v) = kv.split_once('=')?;
            Some((percent_decode(k), percent_decode(v)))
        })
        .collect()
}

/// Opens `url` in the default browser. Returns false if the launcher failed.
fn open_browser(url: &str) -> bool {
    use std::process::{Command, Stdio};

    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = Command::new("open");
        c.arg(url);
        c
    };
    #[cfg(target_os = "windows")]
    let mut cmd = {
        // `start` would treat `&` in the URL as a command separator.
        let mut c = Command::new("rundll32");
        c.args(["url.dll,FileProtocolHandler", url]);
        c
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut cmd = {
        let mut c = Command::new("xdg-open");
        c.arg(url);
        c
    };

    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Runs the OAuth loopback flow and stores the refresh token.
pub fn run_auth_flow(
    cfg: &GoogleHealthConfig,
    token_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}");
    let state = random_state();

    let auth_url = reqwest::Url::parse_with_params(
        AUTH_URL,
        &[
            ("client_id", cfg.client_id.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("response_type", "code"),
            ("scope", SLEEP_SCOPE),
            ("access_type", "offline"),
            ("prompt", "consent"),
            ("state", state.as_str()),
        ],
    )?;

    if open_browser(auth_url.as_str()) {
        eprintln!("Opened your browser to authorize diary-header.");
        eprintln!("If it did not open, visit the following URL manually:\n");
    } else {
        eprintln!("Open the following URL in your browser to authorize diary-header:\n");
    }
    eprintln!("{auth_url}\n");
    eprintln!("Waiting for authorization on {redirect_uri} ...");

    let (mut stream, _) = listener.accept()?;
    let mut request_line = String::new();
    BufReader::new(&stream).read_line(&mut request_line)?;

    // e.g. "GET /?state=...&code=...&scope=... HTTP/1.1"
    let path = request_line.split_whitespace().nth(1).unwrap_or("");
    let params = parse_query(path.split_once('?').map(|(_, q)| q).unwrap_or(""));

    let result = (|| -> Result<String, Box<dyn std::error::Error>> {
        if let Some(err) = params.get("error") {
            return Err(format!("authorization failed: {err}").into());
        }
        if params.get("state") != Some(&state) {
            return Err("authorization failed: state mismatch".into());
        }
        params
            .get("code")
            .cloned()
            .ok_or_else(|| "authorization failed: no code returned".into())
    })();

    let message = match &result {
        Ok(_) => "Authorization complete. You can close this window.",
        Err(_) => "Authorization failed. Check the terminal for details.",
    };
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        message.len(),
        message
    );
    let code = result?;

    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(TOKEN_URL)
        .form(&[
            ("client_id", cfg.client_id.as_str()),
            ("client_secret", cfg.client_secret.as_str()),
            ("code", code.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
        ])
        .send()?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("token exchange failed ({status}): {body}").into());
    }
    let token = resp.json::<TokenResponse>()?;
    let refresh_token = token
        .refresh_token
        .ok_or("no refresh token returned; revoke access and try again")?;

    let json = serde_json::to_string_pretty(&StoredToken { refresh_token })?;
    fs::write(token_path, json)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(token_path, fs::Permissions::from_mode(0o600))?;
    }
    eprintln!("Saved Google Health token to: {}", token_path.display());
    Ok(())
}

// ---- Fetching -----------------------------------------------------------

/// Fetches the main sleep session that ended on `date` (i.e. the night before `date`).
pub fn fetch_sleep(
    cfg: &GoogleHealthConfig,
    token_path: &Path,
    date: NaiveDate,
) -> Result<Option<SleepBreakdown>, Box<dyn std::error::Error>> {
    let stored: StoredToken =
        serde_json::from_str(&fs::read_to_string(token_path).map_err(|_| {
            format!(
                "Google Health token not found at {}. Run `diary-header auth` first.",
                token_path.display()
            )
        })?)?;
    let access_token = refresh_access_token(cfg, &stored.refresh_token)?;

    let next_date = date + Duration::days(1);
    let filter = format!(
        "sleep.interval.civil_end_time >= \"{}\" AND sleep.interval.civil_end_time < \"{}\"",
        date.format("%Y-%m-%d"),
        next_date.format("%Y-%m-%d")
    );

    let client = reqwest::blocking::Client::new();
    let mut sleeps = Vec::new();
    let mut page_token: Option<String> = None;
    loop {
        let mut query: Vec<(&str, &str)> = vec![("filter", filter.as_str()), ("pageSize", "25")];
        if let Some(t) = page_token.as_deref() {
            query.push(("pageToken", t));
        }
        let resp = client
            .get(SLEEP_RECONCILE_URL)
            .bearer_auth(&access_token)
            .header("Accept", "application/json")
            .query(&query)
            .send()?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("Google Health API error ({status}): {body}").into());
        }
        let page = resp.json::<ReconcileResponse>()?;
        sleeps.extend(page.data_points.into_iter().filter_map(|dp| dp.sleep));
        match page.next_page_token {
            Some(t) if !t.is_empty() => page_token = Some(t),
            _ => break,
        }
    }

    Ok(select_main_sleep(&sleeps).and_then(Sleep::breakdown))
}

// ---- Formatting ---------------------------------------------------------

fn format_minutes(minutes: i64, lang: &str) -> String {
    let (h, m) = (minutes / 60, minutes % 60);
    match (lang, h) {
        ("en", 0) => format!("{m}m"),
        ("en", _) => format!("{h}h{m:02}m"),
        (_, 0) => format!("{m}分"),
        (_, _) => format!("{h}時間{m}分"),
    }
}

pub fn format_breakdown(b: &SleepBreakdown, lang: &str) -> String {
    let labels = if lang == "en" {
        ["Total", "Deep", "Light", "REM", "Falling asleep", "Awake"]
    } else {
        ["合計", "深い", "浅い", "レム", "寝付けず", "覚醒"]
    };
    let values = [b.total, b.deep, b.light, b.rem, b.to_fall_asleep, b.awake];
    labels
        .iter()
        .zip(values)
        .map(|(label, v)| format!("{} {}", label, format_minutes(v, lang)))
        .collect::<Vec<_>>()
        .join(" / ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "dataPoints": [
        {
          "dataPointName": "nap",
          "sleep": {
            "metadata": {"nap": true},
            "summary": {"minutesInSleepPeriod": "40", "stagesSummary": []}
          }
        },
        {
          "dataPointName": "main",
          "sleep": {
            "type": "STAGES",
            "metadata": {"stagesStatus": "SUCCEEDED", "processed": true, "main": true},
            "summary": {
              "minutesInSleepPeriod": "464",
              "minutesAfterWakeUp": "0",
              "minutesToFallAsleep": "12",
              "minutesAsleep": "407",
              "minutesAwake": "57",
              "stagesSummary": [
                {"type": "AWAKE", "minutes": "56", "count": "12"},
                {"type": "LIGHT", "minutes": "198", "count": "19"},
                {"type": "DEEP", "minutes": "114", "count": "10"},
                {"type": "REM", "minutes": "94", "count": "4"}
              ]
            }
          }
        }
      ]
    }"#;

    #[test]
    fn parses_and_selects_main_sleep() {
        let resp: ReconcileResponse = serde_json::from_str(SAMPLE).unwrap();
        let sleeps: Vec<Sleep> = resp
            .data_points
            .into_iter()
            .filter_map(|d| d.sleep)
            .collect();
        let b = select_main_sleep(&sleeps)
            .and_then(Sleep::breakdown)
            .unwrap();
        assert_eq!(
            b,
            SleepBreakdown {
                total: 407,
                deep: 114,
                light: 198,
                rem: 94,
                to_fall_asleep: 12,
                awake: 56,
            }
        );
    }

    #[test]
    fn falls_back_to_longest_non_nap() {
        let json = r#"[
          {"metadata": {"nap": true}, "summary": {"minutesInSleepPeriod": "500"}},
          {"summary": {"minutesInSleepPeriod": "300", "minutesAwake": "20"}},
          {"summary": {"minutesInSleepPeriod": "100"}}
        ]"#;
        let sleeps: Vec<Sleep> = serde_json::from_str(json).unwrap();
        let s = select_main_sleep(&sleeps).unwrap();
        assert_eq!(s.period_minutes(), 300);
        assert_eq!(s.breakdown().unwrap().awake, 20);
        // Falls back to the sum of stages when minutesAsleep is missing
        assert_eq!(s.breakdown().unwrap().total, 0);
    }

    #[test]
    fn formats_breakdown() {
        let b = SleepBreakdown {
            total: 406,
            deep: 114,
            light: 198,
            rem: 94,
            to_fall_asleep: 0,
            awake: 56,
        };
        assert_eq!(
            format_breakdown(&b, "ja"),
            "合計 6時間46分 / 深い 1時間54分 / 浅い 3時間18分 / レム 1時間34分 / 寝付けず 0分 / 覚醒 56分"
        );
        assert_eq!(
            format_breakdown(&b, "en"),
            "Total 6h46m / Deep 1h54m / Light 3h18m / REM 1h34m / Falling asleep 0m / Awake 56m"
        );
    }

    #[test]
    fn decodes_query() {
        let q = parse_query("state=abc&code=4%2F0Ab_c&scope=a+b");
        assert_eq!(q["code"], "4/0Ab_c");
        assert_eq!(q["scope"], "a b");
    }
}
