use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};

/// Server-side usage data from the Anthropic OAuth usage API.
/// Cached to ~/.cctop-usage.json and refreshed every ~5 minutes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageData {
    pub session_pct: f64,
    #[serde(default)]
    pub session_reset: String,

    pub weekly_pct: f64,
    #[serde(default)]
    pub weekly_reset: String,

    #[serde(default)]
    pub extra_pct: f64,
    #[serde(default)]
    pub extra_spent: String,
    #[serde(default)]
    pub extra_reset: String,

    #[serde(default = "default_epoch")]
    pub updated_at: DateTime<Utc>,
}

fn default_epoch() -> DateTime<Utc> {
    DateTime::UNIX_EPOCH
}

impl Default for UsageData {
    fn default() -> Self {
        Self {
            session_pct: 0.0,
            session_reset: String::new(),
            weekly_pct: 0.0,
            weekly_reset: String::new(),
            extra_pct: 0.0,
            extra_spent: String::new(),
            extra_reset: String::new(),
            updated_at: DateTime::UNIX_EPOCH,
        }
    }
}

impl UsageData {
    pub fn is_available(&self) -> bool {
        self.updated_at != DateTime::UNIX_EPOCH
    }

    /// Stale after 5 minutes (API data changes continuously)
    pub fn is_stale(&self) -> bool {
        if !self.is_available() {
            return true;
        }
        let age = Utc::now() - self.updated_at;
        age.num_minutes() >= 5
    }

    pub fn age_str(&self) -> String {
        if !self.is_available() {
            return "never".to_string();
        }
        let age = Utc::now() - self.updated_at;
        let secs = age.num_seconds();
        if secs < 60 {
            format!("{}s ago", secs)
        } else {
            format!("{}m ago", age.num_minutes())
        }
    }
}

// ── Credentials ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Credentials {
    claude_ai_oauth: OAuthCreds,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OAuthCreds {
    access_token: String,
}

fn read_access_token() -> Option<String> {
    let path = dirs::home_dir()?.join(".claude").join(".credentials.json");
    let data = fs::read_to_string(path).ok()?;
    let creds: Credentials = serde_json::from_str(&data).ok()?;
    Some(creds.claude_ai_oauth.access_token)
}

// ── API response ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ApiUsage {
    five_hour: Option<UsageWindow>,
    seven_day: Option<UsageWindow>,
    extra_usage: Option<ExtraUsage>,
}

#[derive(Deserialize)]
struct UsageWindow {
    utilization: f64,
    resets_at: Option<String>,
}

#[derive(Deserialize)]
struct ExtraUsage {
    utilization: f64,
    used_credits: Option<f64>,
    monthly_limit: Option<f64>,
}

// ── Public interface ──────────────────────────────────────────────────────────

fn usage_file_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".cctop-usage.json"))
}

pub fn load_cached_usage() -> UsageData {
    let path = match usage_file_path() {
        Some(p) => p,
        None => return UsageData::default(),
    };
    let data = match fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return UsageData::default(),
    };
    serde_json::from_str(&data).unwrap_or_default()
}

fn save_usage(data: &UsageData) {
    if let Some(path) = usage_file_path() {
        if let Ok(json) = serde_json::to_string_pretty(data) {
            let _ = fs::write(path, json);
        }
    }
}

/// Format a UTC ISO8601 reset timestamp into a human-readable local-style string.
fn format_reset(iso: &str) -> String {
    // Parse and convert to local timezone
    // Input: "2026-03-20T10:00:01.122697+00:00"
    if let Ok(dt) = DateTime::parse_from_rfc3339(iso) {
        let utc: DateTime<Utc> = dt.into();
        let local = utc.with_timezone(&Local);
        // Show as "Mar 20, 10:00" in local time
        local.format("%b %d, %H:%M").to_string()
    } else {
        iso.to_string()
    }
}

/// Fetch live usage from Anthropic's OAuth API.
/// Returns Ok(UsageData) on success, Err(message) on failure.
pub fn fetch_api_usage() -> Result<UsageData, String> {
    let token = read_access_token()
        .ok_or_else(|| "No OAuth token in ~/.claude/.credentials.json".to_string())?;

    let response = ureq::get("https://api.anthropic.com/api/oauth/usage")
        .set("Authorization", &format!("Bearer {}", token))
        .set("anthropic-beta", "oauth-2025-04-20")
        .set("User-Agent", "cctop/1.0.0")
        .call()
        .map_err(|e| format!("API request failed: {}", e))?;

    let api: ApiUsage = response
        .into_json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let session_pct = api.five_hour.as_ref().map(|w| w.utilization).unwrap_or(0.0);
    let session_reset = api
        .five_hour
        .as_ref()
        .and_then(|w| w.resets_at.as_deref())
        .map(format_reset)
        .unwrap_or_default();

    let weekly_pct = api.seven_day.as_ref().map(|w| w.utilization).unwrap_or(0.0);
    let weekly_reset = api
        .seven_day
        .as_ref()
        .and_then(|w| w.resets_at.as_deref())
        .map(format_reset)
        .unwrap_or_default();

    let (extra_pct, extra_spent, extra_reset) = if let Some(ex) = &api.extra_usage {
        let spent = match (ex.used_credits, ex.monthly_limit) {
            (Some(used), Some(limit)) => format!("${:.2} / ${:.2}", used / 100.0, limit / 100.0),
            _ => String::new(),
        };
        (ex.utilization, spent, String::new())
    } else {
        (0.0, String::new(), String::new())
    };

    let data = UsageData {
        session_pct,
        session_reset,
        weekly_pct,
        weekly_reset,
        extra_pct,
        extra_spent,
        extra_reset,
        updated_at: Utc::now(),
    };

    save_usage(&data);
    Ok(data)
}

// ── App-level fetch with cooldown ─────────────────────────────────────────────

/// Wraps the API fetch with a cooldown so we don't hammer the API.
pub struct UsageFetcher {
    pub data: UsageData,
    last_attempt: Option<Instant>,
    cooldown_secs: u64,
    pub last_error: Option<String>,
}

impl UsageFetcher {
    pub fn new() -> Self {
        Self {
            data: load_cached_usage(),
            last_attempt: None,
            cooldown_secs: 300, // 5 minutes
            last_error: None,
        }
    }

    /// Adjust cooldown based on whether sessions are actively running.
    pub fn set_active_mode(&mut self, has_running: bool) {
        self.cooldown_secs = if has_running { 60 } else { 300 };
    }

    /// Refresh if stale and cooldown has elapsed. Returns true if data changed.
    pub fn maybe_refresh(&mut self) -> bool {
        let should_attempt = match self.last_attempt {
            None => true,
            Some(t) => t.elapsed().as_secs() >= self.cooldown_secs,
        };

        if !should_attempt {
            return false;
        }

        self.last_attempt = Some(Instant::now());

        match fetch_api_usage() {
            Ok(new_data) => {
                self.data = new_data;
                self.last_error = None;
                true
            }
            Err(e) => {
                self.last_error = Some(e);
                false
            }
        }
    }

    /// Force immediate refresh regardless of cooldown.
    pub fn force_refresh(&mut self) {
        self.last_attempt = None;
        self.maybe_refresh();
    }
}

// ── Legacy: parse pasted /usage text ─────────────────────────────────────────

pub fn parse_usage_text(text: &str) -> UsageData {
    let mut data = UsageData {
        updated_at: Utc::now(),
        ..Default::default()
    };

    let lines: Vec<&str> = text.lines().map(|l| l.trim()).collect();

    enum Section { None, Session, Weekly, Extra }
    let mut section = Section::None;

    for line in &lines {
        let lower = line.to_lowercase();

        if lower.contains("current session") {
            section = Section::Session;
            continue;
        } else if lower.contains("current week") {
            section = Section::Weekly;
            continue;
        } else if lower.contains("extra usage") {
            section = Section::Extra;
            continue;
        }

        if let Some(pct) = extract_percentage(line) {
            match section {
                Section::Session => data.session_pct = pct,
                Section::Weekly => data.weekly_pct = pct,
                Section::Extra => data.extra_pct = pct,
                Section::None => {}
            }
            continue;
        }

        if lower.starts_with("resets ") {
            let reset_text = line[7..].trim().to_string();
            match section {
                Section::Session => data.session_reset = reset_text,
                Section::Weekly => data.weekly_reset = reset_text,
                Section::Extra => data.extra_reset = reset_text,
                Section::None => {}
            }
            continue;
        }

        if line.contains("spent") && line.contains('$') {
            if let Some(dot_pos) = line.find("spent") {
                data.extra_spent = line[..dot_pos].trim().to_string();
            }
            if let Some(resets_pos) = lower.find("resets ") {
                data.extra_reset = line[resets_pos + 7..].trim().to_string();
            }
        }
    }

    data
}

fn extract_percentage(line: &str) -> Option<f64> {
    for word in line.split_whitespace() {
        if word.ends_with('%') {
            if let Ok(pct) = word.trim_end_matches('%').parse::<f64>() {
                return Some(pct);
            }
        }
    }
    None
}

pub fn update_usage_interactive() {
    println!("\n  cctop — Update Usage Data\n");

    // Try API first
    println!("  Trying automatic fetch from Anthropic API...");
    match fetch_api_usage() {
        Ok(data) => {
            println!("  ✓ Fetched live data:");
            println!("    Session: {:.0}% — Resets {}", data.session_pct, data.session_reset);
            println!("    Weekly:  {:.0}% — Resets {}", data.weekly_pct, data.weekly_reset);
            println!("    Extra:   {:.1}% {}", data.extra_pct, data.extra_spent);
            println!("\n  Saved to ~/.cctop-usage.json");
            return;
        }
        Err(e) => {
            println!("  ✗ Auto-fetch failed: {}", e);
            println!("  Falling back to manual paste...\n");
        }
    }

    println!("  Paste the output of `/usage` from Claude Code below.");
    println!("  Press Enter twice (empty line) when done:\n");

    let mut input = String::new();
    let mut empty_count = 0;

    loop {
        let mut line = String::new();
        if std::io::stdin().read_line(&mut line).is_err() {
            break;
        }
        if line.trim().is_empty() {
            empty_count += 1;
            if empty_count >= 2 { break; }
        } else {
            empty_count = 0;
        }
        input.push_str(&line);
    }

    if input.trim().is_empty() {
        println!("  No input received.");
        return;
    }

    let data = parse_usage_text(&input);
    println!("\n  Parsed:");
    println!("    Session: {:.0}% — Resets {}", data.session_pct, data.session_reset);
    println!("    Weekly:  {:.0}% — Resets {}", data.weekly_pct, data.weekly_reset);
    println!("    Extra:   {:.0}% {} — Resets {}", data.extra_pct, data.extra_spent, data.extra_reset);

    save_usage(&data);
    println!("\n  Saved to ~/.cctop-usage.json");
}
