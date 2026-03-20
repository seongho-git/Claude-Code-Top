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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortMode {
    Ctx,
    Cost,
    Duration,
}

pub struct App {
    pub mode: AppMode,
    pub sessions: Vec<Session>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub plan: PlanType,
    pub weekly_cost: f64,
    pub weekly_tokens: u64,
    pub daily_cost: f64,
    pub monthly_cost: f64,
    pub monthly_saved: f64,
    pub active_sessions: usize,
    pub sparkline_data: Vec<u64>,
    pub sort_mode: SortMode,
    pub should_quit: bool,
    pub cache: JsonlCache,
    pub sys: sysinfo::System,
    pub last_refresh: Instant,
}

impl App {
    pub fn new(plan: PlanType) -> Self {
        let mut app = App {
            mode: AppMode::Normal,
            sessions: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            plan,
            weekly_cost: 0.0,
            weekly_tokens: 0,
            daily_cost: 0.0,
            monthly_cost: 0.0,
            monthly_saved: 0.0,
            active_sessions: 0,
            sparkline_data: vec![0; 120], // Store some historical data points
            sort_mode: SortMode::Ctx,
            should_quit: false,
            cache: JsonlCache::new(),
            sys: sysinfo::System::new_all(),
            last_refresh: Instant::now(),
        };

        app.refresh_data();
        app
    }

    pub fn refresh_data(&mut self) {
        self.sys.refresh_all();
        let scan = scan_sessions(&mut self.cache, &mut self.sys);
        self.sessions = scan.sessions;

        // Recompute totals
        self.weekly_cost = self.sessions.iter().map(|s| s.total_cost).sum();
        self.monthly_cost = scan.month.cost;
        self.daily_cost = scan.today.cost;
        self.monthly_saved = scan.month.saved;
        self.weekly_tokens = self
            .sessions
            .iter()
            .map(|s| s.total_usage.total_output())
            .sum();
        self.active_sessions = self.sessions.iter().filter(|s| s.is_active).count();

        // Update sparkline
        let current_total_ctx = self
            .sessions
            .iter()
            .filter(|s| s.is_active)
            .map(|s| s.total_usage.total_input_all())
            .sum();
        self.sparkline_data.push(current_total_ctx);
        if self.sparkline_data.len() > 120 {
            self.sparkline_data.remove(0);
        }

        self.apply_sorting();

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

    pub fn apply_sorting(&mut self) {
        match self.sort_mode {
            SortMode::Cost => self.sort_by_cost(),
            SortMode::Ctx => self.sort_by_ctx(),
            SortMode::Duration => self.sort_by_duration(),
        }
    }

    pub fn toggle_sort(&mut self) {
        self.sort_mode = match self.sort_mode {
            SortMode::Ctx => SortMode::Cost,
            SortMode::Cost => SortMode::Duration,
            SortMode::Duration => SortMode::Ctx,
        };
        self.apply_sorting();
    }

    pub fn refresh_interval(&self) -> Duration {
        if self.sessions.iter().any(|s| s.is_active) {
            Duration::from_secs(2) // 2초 갱신 (명세)
        } else {
            Duration::from_secs(5)
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

    pub fn sort_by_ctx(&mut self) {
        self.sessions.sort_by(|a, b| {
            b.total_usage
                .total_input_all()
                .cmp(&a.total_usage.total_input_all())
        });
    }

    pub fn sort_by_cost(&mut self) {
        self.sessions.sort_by(|a, b| {
            b.total_cost
                .partial_cmp(&a.total_cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    pub fn sort_by_duration(&mut self) {
        self.sessions.sort_by(|a, b| {
            let dur_a = a.last_activity - a.first_activity;
            let dur_b = b.last_activity - b.first_activity;
            dur_b.cmp(&dur_a)
        });
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
