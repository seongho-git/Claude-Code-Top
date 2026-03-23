use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};

use crate::paths::{app_dir, ensure_app_dir, legacy_usage_cache_path, usage_cache_path};

/// Server-side usage data from the Anthropic OAuth usage API.
/// Cached to ~/.claude-code-top/usage.json and refreshed every ~5 minutes.
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
    usage_cache_path()
}

fn load_usage_from_path(path: &Path) -> Option<UsageData> {
    let data = fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn load_cached_usage() -> UsageData {
    [usage_file_path(), legacy_usage_cache_path()]
        .into_iter()
        .flatten()
        .find_map(|path| load_usage_from_path(&path))
        .unwrap_or_default()
}

fn save_usage(data: &UsageData) {
    if ensure_app_dir().is_some() {
        if let Some(path) = usage_file_path() {
            if let Ok(json) = serde_json::to_string_pretty(data) {
                let _ = fs::write(path, json);
            }
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

pub fn fetch_usage_via_script() -> Result<UsageData, String> {
    let claude_code_top_dir = app_dir().ok_or("Could not find home directory")?;

    // Prefer a script next to the current executable, then the repo-local copy
    // when developing via `cargo run`, and finally the installed default path.
    let exe_local_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|dir| dir.join("update.sh")));
    let cwd_local_path = std::env::current_dir()
        .ok()
        .map(|dir| dir.join("update.sh"));
    let installed_path = claude_code_top_dir.join("update.sh");

    let script_path = [exe_local_path, cwd_local_path, Some(installed_path)]
        .into_iter()
        .flatten()
        .find(|path| path.exists())
        .ok_or_else(|| {
            "update.sh not found next to the executable, in the current directory, or in ~/.claude-code-top".to_string()
        })?;

    let shell = std::env::var("SHELL")
        .ok()
        .filter(|s| Path::new(s).exists())
        .unwrap_or_else(|| "sh".to_string());
    let current_dir = std::env::current_dir()
        .map_err(|e| format!("Failed to resolve current directory for update.sh: {}", e))?;

    let output = std::process::Command::new(&shell)
        .arg(&script_path)
        .current_dir(current_dir)
        .output()
        .map_err(|e| format!("Failed to execute update.sh via {}: {}", shell, e))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "update.sh failed with status: {}. Error: {}",
            output.status, err_msg
        ));
    }

    let text = String::from_utf8_lossy(&output.stdout);

    let _ = ensure_app_dir();
    // Save raw parsing info text into the folder
    let _ = fs::write(claude_code_top_dir.join("usage_raw.txt"), text.as_ref());

    let mut data = parse_usage_text(&text);
    // Ensure updated_at is always current
    data.updated_at = Utc::now();
    save_usage(&data);

    // Save the parsed data to the folder for info/debugging
    if let Ok(json) = serde_json::to_string_pretty(&data) {
        let _ = fs::write(claude_code_top_dir.join("usage_parsed.json"), json);
    }

    Ok(data)
}

pub fn has_credentials() -> bool {
    dirs::home_dir()
        .map(|h| h.join(".claude").join(".credentials.json").exists())
        .unwrap_or(false)
}

pub fn fetch_api_usage_with_fallback() -> Result<UsageData, String> {
    if has_credentials() {
        // Credentials exist: try API first; only fall back to tmux script on error
        match fetch_api_usage() {
            Ok(data) => return Ok(data),
            Err(api_err) => {
                // API failed despite having credentials — try tmux script, then give up
                match fetch_usage_via_script() {
                    Ok(data) => return Ok(data),
                    Err(script_err) => {
                        return Err(format!(
                            "API error: {} | Script fallback error: {}",
                            api_err, script_err
                        ));
                    }
                }
            }
        }
    }

    // No credentials: go straight to tmux script (no API attempt)
    fetch_usage_via_script()
}

// ── App-level fetch with cooldown ─────────────────────────────────────────────

use std::sync::{Arc, Mutex};

type FetchResult = Arc<Mutex<Option<Result<UsageData, String>>>>;

/// Wraps the fetch with a cooldown and runs it on a background thread
/// so the TUI never blocks waiting for the script (which can take ~10 s).
pub struct UsageFetcher {
    pub data: UsageData,
    last_attempt: Option<Instant>,
    cooldown_secs: u64,
    pub last_error: Option<String>,
    /// Shared slot filled by the background thread when the fetch completes.
    pending: Option<FetchResult>,
}

impl UsageFetcher {
    pub fn new() -> Self {
        Self {
            data: load_cached_usage(),
            last_attempt: None,
            cooldown_secs: 300, // 5 minutes
            last_error: None,
            pending: None,
        }
    }

    /// Adjust cooldown based on whether sessions are actively running.
    pub fn set_active_mode(&mut self, has_running: bool) {
        self.cooldown_secs = if has_running { 60 } else { 300 };
    }

    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// Non-blocking refresh:
    /// 1. If a background fetch is in flight, check whether it finished and
    ///    harvest the result.
    /// 2. If no fetch is running and the cooldown has elapsed, spawn a new one.
    /// Returns true if `self.data` was updated.
    pub fn maybe_refresh(&mut self) -> bool {
        // ── Harvest completed background fetch ────────────────────────────
        if let Some(slot) = self.pending.clone() {
            if let Ok(mut guard) = slot.try_lock() {
                if let Some(result) = guard.take() {
                    self.pending = None;
                    match result {
                        Ok(mut new_data) => {
                            new_data.updated_at = Utc::now();
                            self.data = new_data;
                            self.last_error = None;
                            return true;
                        }
                        Err(e) => {
                            self.last_error = Some(e);
                        }
                    }
                }
            }
            // Still in flight if we didn't just clear it above
            if self.pending.is_some() {
                return false;
            }
        }

        // ── Decide whether to kick off a new fetch ────────────────────────
        let should_attempt = match self.last_attempt {
            None => true,
            Some(t) => t.elapsed().as_secs() >= self.cooldown_secs,
        };

        if !should_attempt {
            return false;
        }

        self.last_attempt = Some(Instant::now());

        // Spawn background thread; result is placed into the shared slot
        let slot: FetchResult = Arc::new(Mutex::new(None));
        let slot_clone = Arc::clone(&slot);
        std::thread::spawn(move || {
            let result = fetch_api_usage_with_fallback();
            if let Ok(mut guard) = slot_clone.lock() {
                *guard = Some(result);
            }
        });
        self.pending = Some(slot);

        false
    }

    /// Force immediate refresh: reset cooldown and spawn a new background fetch
    /// (cancels any in-flight fetch by dropping its slot).
    pub fn force_refresh(&mut self) {
        self.last_attempt = None;
        self.pending = None; // drop in-flight fetch
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

    enum Section {
        None,
        Session,
        Weekly,
        Extra,
    }
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
    println!("  Trying automatic fetch from Anthropic API or fallback script...");
    match fetch_api_usage_with_fallback() {
        Ok(data) => {
            println!("  ✓ Fetched live data:");
            println!(
                "    Session: {:.0}% — Resets {}",
                data.session_pct, data.session_reset
            );
            println!(
                "    Weekly:  {:.0}% — Resets {}",
                data.weekly_pct, data.weekly_reset
            );
            println!("    Extra:   {:.1}% {}", data.extra_pct, data.extra_spent);
            println!("\n  Saved to ~/.claude-code-top/usage.json");
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
            if empty_count >= 2 {
                break;
            }
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
    println!(
        "    Session: {:.0}% — Resets {}",
        data.session_pct, data.session_reset
    );
    println!(
        "    Weekly:  {:.0}% — Resets {}",
        data.weekly_pct, data.weekly_reset
    );
    println!(
        "    Extra:   {:.0}% {} — Resets {}",
        data.extra_pct, data.extra_spent, data.extra_reset
    );

    save_usage(&data);
    println!("\n  Saved to ~/.claude-code-top/usage.json");
}
