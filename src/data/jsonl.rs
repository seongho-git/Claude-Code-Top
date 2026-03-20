use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Utc};

use super::types::{AssistantEntry, TokenUsage};

pub type JsonlCache = HashMap<PathBuf, (SystemTime, Vec<AssistantEntry>)>;

/// Discover all JSONL files under a project folder.
/// Includes top-level *.jsonl and <uuid>/subagents/*.jsonl
pub fn discover_jsonl_files(project_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    let entries = match fs::read_dir(project_dir) {
        Ok(e) => e,
        Err(_) => return files,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("jsonl") && path.is_file() {
            files.push(path);
        } else if path.is_dir() {
            // Check for subagents/ subdirectory
            let subagents = path.join("subagents");
            if subagents.is_dir() {
                if let Ok(sub_entries) = fs::read_dir(&subagents) {
                    for sub_entry in sub_entries.flatten() {
                        let sub_path = sub_entry.path();
                        if sub_path.extension().and_then(|e| e.to_str()) == Some("jsonl")
                            && sub_path.is_file()
                        {
                            files.push(sub_path);
                        }
                    }
                }
            }
        }
    }

    files
}

/// Parse a single JSONL file, using cache if mtime unchanged.
pub fn parse_jsonl_file(path: &Path, cache: &mut JsonlCache) -> Vec<AssistantEntry> {
    let mtime = match fs::metadata(path).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };

    if let Some((cached_mtime, cached_entries)) = cache.get(path) {
        if *cached_mtime == mtime {
            return cached_entries.clone();
        }
    }

    let entries = parse_jsonl_file_inner(path);
    cache.insert(path.to_path_buf(), (mtime, entries.clone()));
    entries
}

fn parse_jsonl_file_inner(path: &Path) -> Vec<AssistantEntry> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let reader = BufReader::new(file);
    let mut entries_by_id: HashMap<String, AssistantEntry> = HashMap::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        // Fast-skip: only parse lines that contain assistant type
        if !line.contains("\"type\":\"assistant\"") {
            continue;
        }

        if let Some(entry) = parse_assistant_line(&line) {
            // Dedup: keep entry with highest output_tokens for same message ID
            let should_insert = entries_by_id
                .get(&entry.message_id)
                .map_or(true, |existing| {
                    entry.usage.output_tokens > existing.usage.output_tokens
                });

            if should_insert {
                entries_by_id.insert(entry.message_id.clone(), entry);
            }
        }
    }

    entries_by_id.into_values().collect()
}

fn parse_assistant_line(line: &str) -> Option<AssistantEntry> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;

    // Must be type: assistant
    if v.get("type")?.as_str()? != "assistant" {
        return None;
    }

    let message = v.get("message")?;
    let message_id = message.get("id")?.as_str()?.to_string();
    let model = message.get("model")?.as_str()?.to_string();

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

    // Check for thinking content
    let has_thinking = message
        .get("content")
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .any(|item| item.get("type").and_then(|t| t.as_str()) == Some("thinking"))
        })
        .unwrap_or(false);

    // Parse timestamp
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

/// Get the mtime of a JSONL file as DateTime<Utc>.
pub fn file_mtime(path: &Path) -> Option<DateTime<Utc>> {
    let mtime = fs::metadata(path).ok()?.modified().ok()?;
    let duration = mtime.duration_since(SystemTime::UNIX_EPOCH).ok()?;
    Some(
        DateTime::from_timestamp(duration.as_secs() as i64, duration.subsec_nanos())
            .unwrap_or_default(),
    )
}
