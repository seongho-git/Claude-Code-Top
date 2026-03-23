use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use sysinfo::System;

use super::jsonl::{file_mtime, parse_jsonl_file, JsonlCache};
use super::pricing::calculate_cost;
use super::types::{Thread, ThreadStatus, TokenUsage};

/// Cache for decoded project paths (folder_name -> decoded_path).
pub type PathCache = HashMap<String, String>;

/// Decode an encoded project folder name back to an absolute path.
pub fn decode_project_path(encoded: &str) -> String {
    if !encoded.starts_with('-') {
        return encoded.to_string();
    }

    let rest = &encoded[1..];
    let parts: Vec<&str> = rest.split('-').collect();

    if parts.is_empty() {
        return format!("/{}", rest);
    }

    let mut result = String::new();
    let mut i = 0;

    while i < parts.len() {
        let mut best_len = 1;
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

/// Scan all project folders and build thread list.
pub fn scan_threads(
    cache: &mut JsonlCache,
    sys: &mut System,
    path_cache: &mut PathCache,
) -> Vec<Thread> {
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
    let active_threshold = now - chrono::Duration::minutes(30);

    let own_pid = std::process::id();
    let mut claude_pids = HashMap::new();
    for (pid, process) in sys.processes() {
        // Exclude cctop itself — its install path (~/.claude-code-top/) contains
        // "claude", which would cause false-positive matches.
        if pid.as_u32() == own_pid {
            continue;
        }
        let name = process.name().to_string_lossy().to_lowercase();
        // Check exe by filename only (not full path) to avoid matching cctop's
        // install directory (~/.claude-code-top/cctop).
        let is_claude = name.contains("claude")
            || name.contains("ccd-cli")
            || process
                .exe()
                .and_then(|e| e.file_name())
                .map(|fname| {
                    let s = fname.to_string_lossy().to_lowercase();
                    s.contains("claude") || s.contains("ccd-cli")
                })
                .unwrap_or(false);
        if is_claude {
            if let Some(cwd) = process.cwd() {
                let cwd_str = cwd.to_string_lossy().to_string();
                let pid_u32 = pid.as_u32();
                // Keep only the most recently started process (highest PID)
                claude_pids
                    .entry(cwd_str)
                    .and_modify(|existing_pid: &mut u32| {
                        if pid_u32 > *existing_pid {
                            *existing_pid = pid_u32;
                        }
                    })
                    .or_insert(pid_u32);
            }
        }
    }

    let mut threads = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let folder_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        let project_path = path_cache
            .entry(folder_name.clone())
            .or_insert_with(|| decode_project_path(&folder_name))
            .clone();
        let active_pid = claude_pids.get(&project_path).copied();

        // One Thread per top-level JSONL file
        for jsonl_file in find_top_level_jsonls(&path) {
            let session_name = jsonl_file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if let Some(t) = build_thread(
                &folder_name,
                &project_path,
                active_pid,
                &[jsonl_file],
                &session_name,
                cache,
                now,
                active_threshold,
            ) {
                threads.push(t);
            }
        }

        // One Thread per UUID subdirectory (with its subagent files)
        for (uuid_name, files) in find_uuid_sessions(&path) {
            if let Some(t) = build_thread(
                &folder_name,
                &project_path,
                active_pid,
                &files,
                &uuid_name,
                cache,
                now,
                active_threshold,
            ) {
                threads.push(t);
            }
        }
    }

    // When a Claude process is running, ALL sessions under the same project
    // get the same PID (matched by CWD). Fix: only the most recently active
    // session per (project, pid) pair keeps the PID; others become Idle.
    deduplicate_active_pid(&mut threads);

    threads.sort_by(|a, b| {
        b.is_active
            .cmp(&a.is_active)
            .then_with(|| b.last_activity.cmp(&a.last_activity))
    });

    threads
}

/// For each (folder_name, pid) group, only keep the PID on the thread with the
/// most recent last_activity.  The rest are demoted to Idle with no PID.
fn deduplicate_active_pid(threads: &mut [Thread]) {
    // Find the best (most recent) thread index per (folder, pid)
    let mut best: HashMap<(String, u32), (usize, DateTime<Utc>)> = HashMap::new();
    for (i, t) in threads.iter().enumerate() {
        if let Some(pid) = t.pid {
            let key = (t.folder_name.clone(), pid);
            let dominated = best
                .get(&key)
                .is_none_or(|(_, prev_ts)| t.last_activity > *prev_ts);
            if dominated {
                best.insert(key, (i, t.last_activity));
            }
        }
    }

    let best_indices: std::collections::HashSet<usize> =
        best.values().map(|(idx, _)| *idx).collect();

    for (i, t) in threads.iter_mut().enumerate() {
        if t.pid.is_some() && !best_indices.contains(&i) {
            t.pid = None;
            t.status = ThreadStatus::Idle;
            t.is_active = t.last_activity >= (Utc::now() - chrono::Duration::minutes(30));
        }
    }
}

/// Returns top-level *.jsonl files directly inside project_dir (not in subdirs).
fn find_top_level_jsonls(project_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(project_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                files.push(p);
            }
        }
    }
    files
}

/// Returns (uuid_name, all_jsonl_files) for each UUID subdirectory.
fn find_uuid_sessions(project_dir: &Path) -> Vec<(String, Vec<PathBuf>)> {
    let mut sessions = Vec::new();
    if let Ok(entries) = fs::read_dir(project_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if !p.is_dir() {
                continue;
            }
            let uuid_name = match p.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            let mut files = Vec::new();
            if let Ok(sub) = fs::read_dir(&p) {
                for se in sub.flatten() {
                    let sp = se.path();
                    if sp.is_file() && sp.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                        files.push(sp);
                    }
                }
            }
            let subagents = p.join("subagents");
            if subagents.is_dir() {
                if let Ok(sub) = fs::read_dir(&subagents) {
                    for se in sub.flatten() {
                        let sp = se.path();
                        if sp.is_file() && sp.extension().and_then(|e| e.to_str()) == Some("jsonl")
                        {
                            files.push(sp);
                        }
                    }
                }
            }
            if !files.is_empty() {
                sessions.push((uuid_name, files));
            }
        }
    }
    sessions
}

fn build_thread(
    folder_name: &str,
    project_path: &str,
    active_pid: Option<u32>,
    jsonl_files: &[PathBuf],
    session_file: &str,
    cache: &mut JsonlCache,
    now: DateTime<Utc>,
    active_threshold: DateTime<Utc>,
) -> Option<Thread> {
    let mut total_usage = TokenUsage::default();
    let mut last_model = String::new();
    let mut has_thinking = false;
    let mut latest_timestamp = DateTime::<Utc>::MIN_UTC;
    let mut first_timestamp = DateTime::<Utc>::MAX_UTC;
    let mut latest_mtime = DateTime::<Utc>::MIN_UTC;
    let mut last_ctx_used: u64 = 0;

    let five_h_ago = now - chrono::Duration::hours(5);
    let week_ago = now - chrono::Duration::days(7);

    let mut all_usage_by_model: HashMap<String, TokenUsage> = HashMap::new();
    let mut window_5h_by_model: HashMap<String, TokenUsage> = HashMap::new();
    let mut week_usage_by_model: HashMap<String, TokenUsage> = HashMap::new();

    let mut window_5h_message_count: u64 = 0;
    let mut earliest_5h_entry: Option<DateTime<Utc>> = None;
    let mut all_user_messages: Vec<String> = Vec::new();
    let mut last_model_cmd: Option<(DateTime<Utc>, String)> = None;
    let mut last_effort_cmd: Option<(DateTime<Utc>, String)> = None;

    for file in jsonl_files {
        if let Some(mtime) = file_mtime(file) {
            if mtime > latest_mtime {
                latest_mtime = mtime;
            }
        }

        let parsed = parse_jsonl_file(file, cache);
        all_user_messages.extend(parsed.user_messages);

        if let Some((ts, ref model)) = parsed.last_model_cmd {
            if last_model_cmd
                .as_ref()
                .is_none_or(|(prev_ts, _)| ts > *prev_ts)
            {
                last_model_cmd = Some((ts, model.clone()));
            }
        }
        if let Some((ts, ref effort)) = parsed.last_effort_cmd {
            if last_effort_cmd
                .as_ref()
                .is_none_or(|(prev_ts, _)| ts > *prev_ts)
            {
                last_effort_cmd = Some((ts, effort.clone()));
            }
        }

        for entry in &parsed.entries {
            all_usage_by_model
                .entry(entry.model.clone())
                .or_default()
                .add(&entry.usage);
            total_usage.add(&entry.usage);

            if entry.timestamp >= five_h_ago {
                window_5h_by_model
                    .entry(entry.model.clone())
                    .or_default()
                    .add(&entry.usage);
                window_5h_message_count += 1;
                if earliest_5h_entry.is_none() || entry.timestamp < earliest_5h_entry.unwrap() {
                    earliest_5h_entry = Some(entry.timestamp);
                }
            }

            if entry.timestamp >= week_ago {
                week_usage_by_model
                    .entry(entry.model.clone())
                    .or_default()
                    .add(&entry.usage);
            }

            if entry.timestamp < first_timestamp {
                first_timestamp = entry.timestamp;
            }

            if entry.timestamp > latest_timestamp {
                latest_timestamp = entry.timestamp;
                last_model = entry.model.clone();
                has_thinking = entry.has_thinking;
                last_ctx_used = entry.usage.total_input_all();
            }
        }
    }

    // Apply model command override if it's newer than the last assistant entry
    if let Some((cmd_ts, ref cmd_model)) = last_model_cmd {
        if cmd_ts > latest_timestamp {
            last_model = cmd_model.clone();
        }
    }

    // Determine effort: from /effort command or inferred from model+thinking
    let last_effort = if let Some((cmd_ts, ref effort)) = last_effort_cmd {
        if cmd_ts > latest_timestamp {
            effort.clone()
        } else {
            infer_effort(&last_model, has_thinking)
        }
    } else {
        infer_effort(&last_model, has_thinking)
    };

    let mut window_5h_usage = TokenUsage::default();
    for usage in window_5h_by_model.values() {
        window_5h_usage.add(usage);
    }

    let mut weekly_cost = 0.0f64;
    for (model, usage) in &week_usage_by_model {
        let (cost, _) = calculate_cost(model, usage);
        weekly_cost += cost;
    }

    if first_timestamp == DateTime::<Utc>::MAX_UTC {
        first_timestamp = latest_mtime;
    }

    let mut total_cost = 0.0f64;
    let mut saved_cost = 0.0f64;
    for (model, usage) in &all_usage_by_model {
        let (cost, saved) = calculate_cost(model, usage);
        total_cost += cost;
        saved_cost += saved;
    }

    let (is_active, status) = if active_pid.is_some() {
        if latest_mtime >= now - chrono::Duration::minutes(5) {
            (true, ThreadStatus::Running)
        } else {
            (true, ThreadStatus::Waiting)
        }
    } else if latest_mtime >= active_threshold {
        (true, ThreadStatus::Idle)
    } else {
        (false, ThreadStatus::Idle)
    };

    let burn_rate = if is_active && total_usage.output_tokens > 0 {
        let mins = (latest_timestamp - first_timestamp).num_minutes() as f64;
        if mins > 0.0 {
            total_usage.total_output() as f64 / mins
        } else {
            0.0
        }
    } else {
        0.0
    };

    let last_activity = if latest_timestamp > DateTime::<Utc>::MIN_UTC {
        latest_timestamp
    } else {
        latest_mtime
    };

    if last_model.is_empty()
        && total_usage.output_tokens == 0
        && latest_mtime == DateTime::<Utc>::MIN_UTC
    {
        return None;
    }

    // Recent commands: last 10 user messages
    let cmd_count = 10.min(all_user_messages.len());
    let recent_commands = if cmd_count > 0 {
        all_user_messages[all_user_messages.len() - cmd_count..].to_vec()
    } else {
        Vec::new()
    };

    Some(Thread {
        pid: active_pid,
        status,
        project_path: project_path.to_string(),
        folder_name: folder_name.to_string(),
        session_file: session_file.to_string(),
        total_usage,
        total_cost,
        saved_cost,
        last_model,
        first_activity: first_timestamp,
        last_activity,
        is_active,
        burn_rate,
        jsonl_files: jsonl_files.to_vec(),
        window_5h_usage,
        window_5h_start: earliest_5h_entry,
        window_5h_message_count,
        weekly_cost,
        per_model_usage: all_usage_by_model,
        recent_commands,
        last_ctx_used,
        last_effort,
    })
}

/// Infer effort level from model name and thinking presence.
fn infer_effort(model: &str, has_thinking: bool) -> String {
    let lower = model.to_lowercase();
    if has_thinking {
        if lower.contains("opus") {
            "max".to_string()
        } else {
            "high".to_string()
        }
    } else if lower.contains("haiku") {
        "low".to_string()
    } else {
        "auto".to_string()
    }
}
