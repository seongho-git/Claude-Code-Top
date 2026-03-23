use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Utc};

use super::types::{AssistantEntry, TokenUsage};

/// Cached parse result for a single JSONL file.
#[derive(Clone)]
pub struct CachedFile {
    pub entries: Vec<AssistantEntry>,
    pub user_messages: Vec<String>,
    /// Most recent /model command: (timestamp, model_id)
    pub last_model_cmd: Option<(DateTime<Utc>, String)>,
    /// Most recent /effort command: (timestamp, effort_level)
    pub last_effort_cmd: Option<(DateTime<Utc>, String)>,
}

pub type JsonlCache = HashMap<PathBuf, (SystemTime, CachedFile)>;

/// Parse a single JSONL file, using cache if mtime unchanged.
pub fn parse_jsonl_file(path: &Path, cache: &mut JsonlCache) -> CachedFile {
    let mtime = match fs::metadata(path).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => {
            return CachedFile {
                entries: Vec::new(),
                user_messages: Vec::new(),
                last_model_cmd: None,
                last_effort_cmd: None,
            }
        }
    };

    if let Some((cached_mtime, cached)) = cache.get(path) {
        if *cached_mtime == mtime {
            return cached.clone();
        }
    }

    let result = do_parse_jsonl(path);
    cache.insert(path.to_path_buf(), (mtime, result.clone()));
    result
}

fn do_parse_jsonl(path: &Path) -> CachedFile {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => {
            return CachedFile {
                entries: Vec::new(),
                user_messages: Vec::new(),
                last_model_cmd: None,
                last_effort_cmd: None,
            }
        }
    };

    let reader = BufReader::new(file);
    let mut entries_by_id: HashMap<String, AssistantEntry> = HashMap::new();
    let mut user_messages: Vec<String> = Vec::new();
    let mut last_model_cmd: Option<(DateTime<Utc>, String)> = None;
    let mut last_effort_cmd: Option<(DateTime<Utc>, String)> = None;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        if line.contains("\"type\":\"assistant\"") {
            if let Some(entry) = parse_assistant_line(&line) {
                let should_insert = entries_by_id.get(&entry.message_id).is_none_or(|existing| {
                    entry.usage.output_tokens > existing.usage.output_tokens
                });
                if should_insert {
                    entries_by_id.insert(entry.message_id.clone(), entry);
                }
            }
        }

        // Parse user messages from both "human" and "user" type entries
        if line.contains("\"type\":\"human\"") || line.contains("\"type\":\"user\"") {
            if let Some(msg) = parse_user_message(&line) {
                user_messages.push(msg);
            }
            // Detect /model and /effort commands
            if let Some(cmd) = parse_model_command(&line) {
                last_model_cmd = Some(cmd);
            }
            if let Some(cmd) = parse_effort_command(&line) {
                last_effort_cmd = Some(cmd);
            }
        }
    }

    CachedFile {
        entries: entries_by_id.into_values().collect(),
        user_messages,
        last_model_cmd,
        last_effort_cmd,
    }
}

fn parse_assistant_line(line: &str) -> Option<AssistantEntry> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;

    if v.get("type")?.as_str()? != "assistant" {
        return None;
    }

    let message = v.get("message")?;
    let message_id = message.get("id")?.as_str()?.to_string();
    let model = message.get("model")?.as_str()?.to_string();

    // Skip synthetic (internal placeholder) messages
    if model.contains("synthetic") {
        return None;
    }

    let usage = message.get("usage")?;
    let token_usage = TokenUsage {
        input_tokens: usage
            .get("input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        output_tokens: usage
            .get("output_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        cache_creation_input_tokens: usage
            .get("cache_creation_input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        cache_read_input_tokens: usage
            .get("cache_read_input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
    };

    let has_thinking = message
        .get("content")
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .any(|item| item.get("type").and_then(|t| t.as_str()) == Some("thinking"))
        })
        .unwrap_or(false);

    let timestamp_str = v.get("timestamp")?.as_str()?;
    let timestamp: DateTime<Utc> = timestamp_str.parse().ok()?;

    Some(AssistantEntry {
        message_id,
        model,
        usage: token_usage,
        has_thinking,
        timestamp,
    })
}

/// Extract user message text from "human" or "user" type JSONL lines.
/// Filters out meta entries (isMeta), system commands, and tool results.
fn parse_user_message(line: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;

    let entry_type = v.get("type")?.as_str()?;
    if entry_type != "human" && entry_type != "user" {
        return None;
    }

    // Skip meta entries (system-generated context)
    if v.get("isMeta").and_then(|v| v.as_bool()).unwrap_or(false) {
        return None;
    }

    let message = v.get("message")?;
    let content = message.get("content")?;

    // Content can be a string or an array of content blocks
    let text = if let Some(s) = content.as_str() {
        s.trim().to_string()
    } else if let Some(arr) = content.as_array() {
        let mut combined = String::new();
        for item in arr {
            if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                if let Some(t) = item.get("text").and_then(|t| t.as_str()) {
                    if !combined.is_empty() {
                        combined.push(' ');
                    }
                    combined.push_str(t.trim());
                }
            }
        }
        combined
    } else {
        return None;
    };

    if text.is_empty() {
        return None;
    }

    // Filter out system/command entries
    if text.starts_with('<')
        && (text.contains("<command-name>")
            || text.contains("<local-command")
            || text.contains("<system-reminder>"))
    {
        return None;
    }

    Some(truncate_line(&text, 200))
}

fn truncate_line(s: &str, max_len: usize) -> String {
    let first_line = s.lines().next().unwrap_or(s);
    let chars: Vec<char> = first_line.chars().collect();
    if chars.len() > max_len {
        let truncated: String = chars[..max_len].iter().collect();
        format!("{}...", truncated)
    } else {
        first_line.to_string()
    }
}

/// Parse model change from stdout: "Set model to \x1b[1mOpus 4.6\x1b[22m"
fn parse_model_command(line: &str) -> Option<(DateTime<Utc>, String)> {
    // Look for the stdout result of a /model command
    if !line.contains("Set model to") {
        return None;
    }

    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    if v.get("type")?.as_str()? != "user" {
        return None;
    }

    let content = v.get("message")?.get("content")?.as_str()?;
    if !content.contains("<local-command-stdout>") || !content.contains("Set model to") {
        return None;
    }

    let stdout_start = content.find("<local-command-stdout>")? + "<local-command-stdout>".len();
    let stdout_end = content.find("</local-command-stdout>")?;
    let stdout = &content[stdout_start..stdout_end];

    // Strip ANSI escape codes
    let clean = strip_ansi(stdout);
    let model_display = clean.strip_prefix("Set model to ")?.trim();

    // Map display name → API model ID
    let model_id = display_name_to_model_id(model_display);

    let ts_str = v.get("timestamp")?.as_str()?;
    let ts: DateTime<Utc> = ts_str.parse().ok()?;

    Some((ts, model_id))
}

/// Parse effort level from /effort command args
fn parse_effort_command(line: &str) -> Option<(DateTime<Utc>, String)> {
    if !line.contains("/effort") {
        return None;
    }

    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    if v.get("type")?.as_str()? != "user" {
        return None;
    }

    let content = v.get("message")?.get("content")?.as_str()?;
    if !content.contains("<command-name>/effort</command-name>") {
        return None;
    }

    // Extract effort from <command-args>X</command-args>
    let args_start = content.find("<command-args>")? + "<command-args>".len();
    let args_end = content.find("</command-args>")?;
    let effort = content[args_start..args_end].trim().to_lowercase();

    if effort.is_empty() {
        return None;
    }

    let ts_str = v.get("timestamp")?.as_str()?;
    let ts: DateTime<Utc> = ts_str.parse().ok()?;

    Some((ts, effort))
}

/// Strip ANSI escape sequences from a string.
fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip until 'm' (end of SGR sequence)
            for c2 in chars.by_ref() {
                if c2 == 'm' {
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Map display model name (e.g. "Opus 4.6") to API model ID (e.g. "claude-opus-4-6").
fn display_name_to_model_id(display: &str) -> String {
    let lower = display.to_lowercase();
    let clean = lower.split('(').next().unwrap_or(&lower).trim();
    format!("claude-{}", clean.replace([' ', '.'], "-"))
}

/// Get the mtime of a JSONL file as DateTime<Utc>.
pub fn file_mtime(path: &Path) -> Option<DateTime<Utc>> {
    let mtime = fs::metadata(path).ok()?.modified().ok()?;
    let duration = mtime.duration_since(SystemTime::UNIX_EPOCH).ok()?;
    Some(
        DateTime::from_timestamp(duration.as_secs() as i64, duration.subsec_nanos())
            .unwrap_or_default(),
    )
}
