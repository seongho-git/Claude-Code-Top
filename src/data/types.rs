use chrono::{DateTime, Utc};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlanType {
    Pro,
    Max5,
    Max20,
    Custom(u64), // Contains custom token limit
}

impl PlanType {
    pub fn token_limit(&self) -> u64 {
        match self {
            PlanType::Pro => 19_000,
            PlanType::Max5 => 88_000,
            PlanType::Max20 => 220_000,
            PlanType::Custom(l) => *l,
        }
    }

    pub fn cost_limit(&self) -> f64 {
        match self {
            PlanType::Pro => 18.0,
            PlanType::Max5 => 35.0,
            PlanType::Max20 => 140.0,
            PlanType::Custom(l) => (*l as f64) / 1000.0, // rough custom cost limit
        }
    }

    pub fn label(&self) -> String {
        match self {
            PlanType::Pro => "Pro".to_string(),
            PlanType::Max5 => "Max5".to_string(),
            PlanType::Max20 => "Max20".to_string(),
            PlanType::Custom(_) => "Custom".to_string(),
        }
    }

    pub fn from_str(s: &str) -> Option<PlanType> {
        let parts: Vec<&str> = s.split(':').collect();
        match parts[0].to_lowercase().as_str() {
            "pro" => Some(PlanType::Pro),
            "max5" => Some(PlanType::Max5),
            "max20" => Some(PlanType::Max20),
            "custom" => {
                let limit = parts.get(1).and_then(|v| v.parse().ok()).unwrap_or(500_000);
                Some(PlanType::Custom(limit))
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModelTier {
    Opus,
    Sonnet,
    Haiku,
}

impl ModelTier {
    pub fn from_model_name(name: &str) -> ModelTier {
        let lower = name.to_lowercase();
        if lower.contains("opus") {
            ModelTier::Opus
        } else if lower.contains("haiku") {
            ModelTier::Haiku
        } else {
            ModelTier::Sonnet
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
}

impl TokenUsage {
    pub fn total_output(&self) -> u64 {
        self.output_tokens
    }

    pub fn total_input_all(&self) -> u64 {
        self.input_tokens + self.cache_creation_input_tokens + self.cache_read_input_tokens
    }

    pub fn total_tokens(&self) -> u64 {
        self.total_input_all() + self.output_tokens
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.total_input_all();
        if total > 0 {
            (self.cache_read_input_tokens as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }

    pub fn add(&mut self, other: &TokenUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cache_creation_input_tokens += other.cache_creation_input_tokens;
        self.cache_read_input_tokens += other.cache_read_input_tokens;
    }
}

#[derive(Debug, Clone)]
pub struct AssistantEntry {
    pub message_id: String,
    pub model: String,
    pub usage: TokenUsage,
    pub has_thinking: bool,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SessionStatus {
    Running,
    Waiting,
    Idle,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EffortLevel {
    Standard,
    Deep,
}

impl EffortLevel {
    pub fn label(&self) -> &'static str {
        match self {
            EffortLevel::Standard => "standard",
            EffortLevel::Deep => "think",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct UsageSummary {
    pub usage: TokenUsage,
    pub cost: f64,
    pub saved: f64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Session {
    pub pid: Option<u32>,
    pub status: SessionStatus,
    pub project_path: String,
    pub folder_name: String,
    pub total_usage: TokenUsage,
    pub total_cost: f64,
    pub saved_cost: f64,
    pub last_model: String,
    pub effort: EffortLevel,
    pub has_thinking: bool,
    pub first_activity: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub is_active: bool,
    pub burn_rate: f64, // tokens per minute
    pub jsonl_files: Vec<PathBuf>,
}
