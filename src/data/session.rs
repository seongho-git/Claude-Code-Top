use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Duration, Utc};

use super::jsonl::{discover_jsonl_files, file_mtime, parse_jsonl_file, JsonlCache};
use super::pricing::calculate_cost;
use super::types::{Session, TokenUsage};

/// Decode an encoded project folder name back to an absolute path.
/// e.g. "-home-seongho-archive-Claude-Code-Usage-Monitor" → "/home/seongho/archive/Claude-Code-Usage-Monitor"
///
/// Greedy algorithm: split on '-', walk left-to-right, try longest runs
/// that form existing directories to handle hyphenated dir names.
pub fn decode_project_path(encoded: &str) -> String {
    // The encoding replaces '/' with '-' and prepends '-' for the root '/'
    // So "-home-seongho-project" means "/home/seongho/project"
    // But hyphens in real dir names are preserved, so we need greedy matching.

    if !encoded.starts_with('-') {
        return encoded.to_string();
    }

    // Remove leading '-' and split by '-'
    let rest = &encoded[1..];
    let parts: Vec<&str> = rest.split('-').collect();

    if parts.is_empty() {
        return format!("/{}", rest);
    }

    let mut result = String::new();
    let mut i = 0;

    while i < parts.len() {
        // Try greedy: longest run of parts joined by '-' that exists as a dir
        let mut best_len = 1; // at minimum, take one part

        // Try joining from parts[i..i+len] for decreasing lengths
        let max_try = parts.len() - i;
        for try_len in (1..=max_try).rev() {
            let candidate_segment = parts[i..i + try_len].join("-");
            let candidate_path = format!("{}/{}", result, candidate_segment);

            if Path::new(&candidate_path).exists() {
                best_len = try_len;
                break;
            }
        }

        let segment = parts[i..i + best_len].join("-");
        result.push('/');
        result.push_str(&segment);
        i += best_len;
    }

    result
}

/// Scan all project folders and build session list.
pub fn scan_sessions(cache: &mut JsonlCache) -> Vec<Session> {
    let claude_dir = match dirs::home_dir() {
        Some(h) => h.join(".claude").join("projects"),
        None => return Vec::new(),
    };

    if !claude_dir.is_dir() {
        return Vec::new();
    }

    let entries = match fs::read_dir(&claude_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let now = Utc::now();
    let week_ago = now - Duration::days(7);
    let active_threshold = now - Duration::minutes(30);

    let mut sessions = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let folder_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        let jsonl_files = discover_jsonl_files(&path);
        if jsonl_files.is_empty() {
            continue;
        }

        let session = build_session(&folder_name, &jsonl_files, cache, week_ago, active_threshold);
        if let Some(s) = session {
            sessions.push(s);
        }
    }

    // Sort: active first, then by last_activity descending
    sessions.sort_by(|a, b| {
        b.is_active
            .cmp(&a.is_active)
            .then_with(|| b.last_activity.cmp(&a.last_activity))
    });

    sessions
}

fn build_session(
    folder_name: &str,
    jsonl_files: &[PathBuf],
    cache: &mut JsonlCache,
    week_ago: chrono::DateTime<Utc>,
    active_threshold: chrono::DateTime<Utc>,
) -> Option<Session> {
    let project_path = decode_project_path(folder_name);

    let mut total_usage = TokenUsage::default();
    let mut total_cost = 0.0f64;
    let mut last_model = String::new();
    let mut has_thinking = false;
    let mut latest_timestamp = chrono::DateTime::<Utc>::MIN_UTC;

    // Track latest file mtime for active status
    let mut latest_mtime = chrono::DateTime::<Utc>::MIN_UTC;

    // Collect per-model usage for cost in weekly window
    let mut weekly_usage_by_model: HashMap<String, TokenUsage> = HashMap::new();

    for file in jsonl_files {
        if let Some(mtime) = file_mtime(file) {
            if mtime > latest_mtime {
                latest_mtime = mtime;
            }
        }

        let entries = parse_jsonl_file(file, cache);

        for entry in &entries {
            // Only count entries within the weekly window for cost/usage totals
            if entry.timestamp >= week_ago {
                let model_usage = weekly_usage_by_model
                    .entry(entry.model.clone())
                    .or_default();
                model_usage.add(&entry.usage);
                total_usage.add(&entry.usage);
            }

            if entry.has_thinking {
                has_thinking = true;
            }

            if entry.timestamp > latest_timestamp {
                latest_timestamp = entry.timestamp;
                last_model = entry.model.clone();
            }
        }
    }

    // Calculate weekly cost by model
    for (model, usage) in &weekly_usage_by_model {
        total_cost += calculate_cost(model, usage);
    }

    // Determine active status from file mtime
    let is_active = latest_mtime >= active_threshold;
    let last_activity = if latest_timestamp > chrono::DateTime::<Utc>::MIN_UTC {
        latest_timestamp
    } else {
        latest_mtime
    };

    // Skip sessions with no assistant entries at all
    if last_model.is_empty() && total_usage.output_tokens == 0 {
        // Still include if there are files, just might not have weekly data
        if latest_mtime == chrono::DateTime::<Utc>::MIN_UTC {
            return None;
        }
    }

    Some(Session {
        project_path,
        folder_name: folder_name.to_string(),
        total_usage,
        total_cost,
        last_model,
        has_thinking,
        last_activity,
        is_active,
        jsonl_files: jsonl_files.to_vec(),
    })
}
