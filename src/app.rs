use std::fs;
use std::time::{Duration, Instant};

use crate::data::jsonl::JsonlCache;
use crate::data::session::scan_sessions;
use crate::data::types::{PlanType, Session};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Normal,
    ConfirmDelete { index: usize },
}

pub struct App {
    pub mode: AppMode,
    pub sessions: Vec<Session>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub plan: PlanType,
    pub weekly_cost: f64,
    pub weekly_tokens: u64,
    pub username: String,
    pub hostname: String,
    pub should_quit: bool,
    pub cache: JsonlCache,
    pub last_refresh: Instant,
}

impl App {
    pub fn new(plan: PlanType) -> Self {
        let username = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "user".to_string());

        let hostname = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "localhost".to_string());

        let mut app = App {
            mode: AppMode::Normal,
            sessions: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            plan,
            weekly_cost: 0.0,
            weekly_tokens: 0,
            username,
            hostname,
            should_quit: false,
            cache: JsonlCache::new(),
            last_refresh: Instant::now(),
        };

        app.refresh_data();
        app
    }

    pub fn refresh_data(&mut self) {
        self.sessions = scan_sessions(&mut self.cache);

        // Recompute weekly totals
        self.weekly_cost = self.sessions.iter().map(|s| s.total_cost).sum();
        self.weekly_tokens = self
            .sessions
            .iter()
            .map(|s| s.total_usage.total_output())
            .sum();

        // Clamp selection
        if !self.sessions.is_empty() {
            if self.selected >= self.sessions.len() {
                self.selected = self.sessions.len() - 1;
            }
        } else {
            self.selected = 0;
        }

        self.last_refresh = Instant::now();
    }

    pub fn refresh_interval(&self) -> Duration {
        if self.sessions.iter().any(|s| s.is_active) {
            Duration::from_secs(10)
        } else {
            Duration::from_secs(60)
        }
    }

    pub fn needs_refresh(&self) -> bool {
        self.last_refresh.elapsed() >= self.refresh_interval()
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if !self.sessions.is_empty() && self.selected < self.sessions.len() - 1 {
            self.selected += 1;
        }
    }

    pub fn request_delete(&mut self) {
        if !self.sessions.is_empty() {
            self.mode = AppMode::ConfirmDelete {
                index: self.selected,
            };
        }
    }

    pub fn confirm_delete(&mut self) {
        if let AppMode::ConfirmDelete { index } = self.mode {
            if index < self.sessions.len() {
                self.delete_session(index);
            }
            self.mode = AppMode::Normal;
            self.refresh_data();
        }
    }

    pub fn cancel_delete(&mut self) {
        self.mode = AppMode::Normal;
    }

    fn delete_session(&self, index: usize) {
        let session = &self.sessions[index];

        for file in &session.jsonl_files {
            let _ = fs::remove_file(file);

            // Clean up empty subagents/ directory
            if let Some(parent) = file.parent() {
                if parent.file_name().and_then(|n| n.to_str()) == Some("subagents") {
                    let _ = fs::remove_dir(parent); // only removes if empty
                    // Also try to remove the parent UUID dir
                    if let Some(uuid_dir) = parent.parent() {
                        let _ = fs::remove_dir(uuid_dir);
                    }
                }
            }
        }
    }
}
