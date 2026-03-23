use std::collections::HashMap;
use std::fs;
use std::time::{Duration, Instant};

use crate::data::jsonl::JsonlCache;
use crate::data::pricing::calculate_cost;
use crate::data::thread::{scan_threads, PathCache};
use crate::data::types::{PlanType, ThreadStatus, TokenUsage};
use crate::data::usage::{UsageData, UsageFetcher};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Normal,
    ConfirmDelete { index: usize },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortColumn {
    Pid,
    Project,
    Status,
    Model,
    Effort,
    Ctx,
    Cache,
    Cost,
    Duration,
}

impl SortColumn {
    pub fn next(self) -> Self {
        match self {
            Self::Pid => Self::Project,
            Self::Project => Self::Status,
            Self::Status => Self::Model,
            Self::Model => Self::Effort,
            Self::Effort => Self::Ctx,
            Self::Ctx => Self::Cache,
            Self::Cache => Self::Cost,
            Self::Cost => Self::Duration,
            Self::Duration => Self::Pid,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Pid => Self::Duration,
            Self::Project => Self::Pid,
            Self::Status => Self::Project,
            Self::Model => Self::Status,
            Self::Effort => Self::Model,
            Self::Ctx => Self::Effort,
            Self::Cache => Self::Ctx,
            Self::Cost => Self::Cache,
            Self::Duration => Self::Cost,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Pid => "PID",
            Self::Project => "PROJECT",
            Self::Status => "STATUS",
            Self::Model => "MODEL",
            Self::Effort => "EFFORT",
            Self::Ctx => "CTX",
            Self::Cache => "CACHE",
            Self::Cost => "COST",
            Self::Duration => "DURATION",
        }
    }
}

pub struct App {
    pub mode: AppMode,
    pub threads: Vec<crate::data::types::Thread>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub plan: PlanType,
    pub active_threads: usize,
    pub sort_column: SortColumn,
    pub should_quit: bool,
    pub cache: JsonlCache,
    pub path_cache: PathCache,
    pub sys: sysinfo::System,
    pub last_refresh: Instant,
    pub window_5h_tokens: u64,
    pub window_5h_messages: u64,
    pub window_5h_reset: Option<chrono::DateTime<chrono::Utc>>,
    pub weekly_cost: f64,
    pub total_cost_all: f64,
    pub total_tokens_all: u64,
    pub lifetime_by_model: Vec<(String, u64, f64)>,
    pub usage_data: UsageData,
    fetcher: UsageFetcher,
}

impl App {
    pub fn new(plan: PlanType) -> Self {
        let mut app = App {
            mode: AppMode::Normal,
            threads: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            plan,
            active_threads: 0,
            sort_column: SortColumn::Ctx,
            should_quit: false,
            cache: JsonlCache::new(),
            path_cache: PathCache::new(),
            sys: sysinfo::System::new(),
            last_refresh: Instant::now(),
            window_5h_tokens: 0,
            window_5h_messages: 0,
            window_5h_reset: None,
            weekly_cost: 0.0,
            total_cost_all: 0.0,
            total_tokens_all: 0,
            lifetime_by_model: Vec::new(),
            usage_data: UsageData::default(),
            fetcher: UsageFetcher::new(),
        };

        app.refresh_data();
        app
    }

    pub fn force_refresh_usage(&mut self) {
        self.fetcher.force_refresh();
        self.usage_data = self.fetcher.data.clone();
    }

    pub fn refresh_data(&mut self) {
        self.sys.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            true,
            sysinfo::ProcessRefreshKind::nothing()
                .with_cwd(sysinfo::UpdateKind::Always)
                .with_exe(sysinfo::UpdateKind::OnlyIfNotSet),
        );
        self.threads = scan_threads(&mut self.cache, &mut self.sys, &mut self.path_cache);
        self.active_threads = self.threads.iter().filter(|t| t.is_active).count();

        // Global 5h rolling window
        let mut global_5h = TokenUsage::default();
        let mut earliest_5h: Option<chrono::DateTime<chrono::Utc>> = None;
        for t in &self.threads {
            global_5h.add(&t.window_5h_usage);
            if let Some(start) = t.window_5h_start {
                if earliest_5h.is_none() || start < earliest_5h.unwrap() {
                    earliest_5h = Some(start);
                }
            }
        }
        self.window_5h_tokens = global_5h.total_input_all() + global_5h.output_tokens;
        self.window_5h_messages = self.threads.iter().map(|t| t.window_5h_message_count).sum();
        self.window_5h_reset = earliest_5h.map(|t| t + chrono::Duration::hours(5));

        self.weekly_cost = self.threads.iter().map(|t| t.weekly_cost).sum();

        // Lifetime totals + per-model breakdown
        let mut all_by_model: HashMap<String, (u64, f64)> = HashMap::new();
        for t in &self.threads {
            for (model, usage) in &t.per_model_usage {
                let tokens = usage.total_input_all() + usage.output_tokens;
                let (cost, _) = calculate_cost(model, usage);
                let entry = all_by_model.entry(model.clone()).or_insert((0, 0.0));
                entry.0 += tokens;
                entry.1 += cost;
            }
        }
        self.total_cost_all = self.threads.iter().map(|t| t.total_cost).sum();
        self.total_tokens_all = all_by_model.values().map(|(t, _)| *t).sum();
        let mut sorted_models: Vec<(String, u64, f64)> = all_by_model
            .into_iter()
            .map(|(m, (t, c))| (m, t, c))
            .collect();
        sorted_models.sort_by(|a, b| b.1.cmp(&a.1));
        self.lifetime_by_model = sorted_models;

        self.apply_sorting();

        if !self.threads.is_empty() {
            if self.selected >= self.threads.len() {
                self.selected = self.threads.len() - 1;
            }
        } else {
            self.selected = 0;
        }

        let has_running = self
            .threads
            .iter()
            .any(|t| t.status == ThreadStatus::Running);
        self.fetcher.set_active_mode(has_running);
        self.fetcher.maybe_refresh();
        self.usage_data = self.fetcher.data.clone();

        self.last_refresh = Instant::now();
    }

    pub fn apply_sorting(&mut self) {
        match self.sort_column {
            SortColumn::Pid => self
                .threads
                .sort_by(|a, b| a.pid.unwrap_or(0).cmp(&b.pid.unwrap_or(0))),
            SortColumn::Project => self
                .threads
                .sort_by(|a, b| a.project_path.cmp(&b.project_path)),
            SortColumn::Status => self
                .threads
                .sort_by(|a, b| status_ord(&b.status).cmp(&status_ord(&a.status))),
            SortColumn::Model => self.threads.sort_by(|a, b| a.last_model.cmp(&b.last_model)),
            SortColumn::Effort => self
                .threads
                .sort_by(|a, b| effort_ord(&b.last_effort).cmp(&effort_ord(&a.last_effort))),
            SortColumn::Ctx => self.threads.sort_by(|a, b| {
                b.total_usage
                    .total_input_all()
                    .cmp(&a.total_usage.total_input_all())
            }),
            SortColumn::Cache => self.threads.sort_by(|a, b| {
                b.total_usage
                    .hit_rate()
                    .partial_cmp(&a.total_usage.hit_rate())
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            SortColumn::Cost => self.threads.sort_by(|a, b| {
                b.total_cost
                    .partial_cmp(&a.total_cost)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            SortColumn::Duration => self.threads.sort_by(|a, b| {
                let dur_a = a.last_activity - a.first_activity;
                let dur_b = b.last_activity - b.first_activity;
                dur_b.cmp(&dur_a)
            }),
        }
    }

    pub fn sort_next(&mut self) {
        self.sort_column = self.sort_column.next();
        self.apply_sorting();
    }

    pub fn sort_prev(&mut self) {
        self.sort_column = self.sort_column.prev();
        self.apply_sorting();
    }

    pub fn toggle_sort(&mut self) {
        self.sort_next();
    }

    pub fn refresh_interval(&self) -> Duration {
        if self.fetcher.has_pending() {
            Duration::from_millis(500)
        } else {
            Duration::from_secs(2)
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
        if !self.threads.is_empty() && self.selected < self.threads.len() - 1 {
            self.selected += 1;
        }
    }

    pub fn request_delete(&mut self) {
        if !self.threads.is_empty() {
            self.mode = AppMode::ConfirmDelete {
                index: self.selected,
            };
        }
    }

    pub fn confirm_delete(&mut self) {
        if let AppMode::ConfirmDelete { index } = self.mode {
            if index < self.threads.len() {
                self.delete_thread(index);
            }
            self.mode = AppMode::Normal;
            self.refresh_data();
        }
    }

    pub fn cancel_delete(&mut self) {
        self.mode = AppMode::Normal;
    }

    fn delete_thread(&self, index: usize) {
        let thread = &self.threads[index];

        // Remove all discovered JSONL files
        for file in &thread.jsonl_files {
            let _ = fs::remove_file(file);

            if let Some(parent) = file.parent() {
                if parent.file_name().and_then(|n| n.to_str()) == Some("subagents") {
                    let _ = fs::remove_dir(parent);
                    if let Some(uuid_dir) = parent.parent() {
                        let _ = fs::remove_dir(uuid_dir);
                    }
                } else {
                    // File inside a UUID dir directly
                    let _ = fs::remove_dir(parent);
                }
            }
        }

        // Try to remove the project folder if it's now empty
        if let Some(home) = dirs::home_dir() {
            let project_dir = home
                .join(".claude")
                .join("projects")
                .join(&thread.folder_name);
            if project_dir.is_dir() {
                // Try removing empty subdirectories
                if let Ok(entries) = fs::read_dir(&project_dir) {
                    for entry in entries.flatten() {
                        if entry.path().is_dir() {
                            let _ = fs::remove_dir(entry.path());
                        }
                    }
                }
                let _ = fs::remove_dir(&project_dir);
            }
        }
    }
}

fn status_ord(status: &ThreadStatus) -> u8 {
    match status {
        ThreadStatus::Running => 3,
        ThreadStatus::Waiting => 2,
        ThreadStatus::Idle => 1,
        ThreadStatus::Error => 0,
    }
}

fn effort_ord(effort: &str) -> u8 {
    match effort {
        "max" => 3,
        "high" => 2,
        "low" => 0,
        _ => 1,
    }
}
