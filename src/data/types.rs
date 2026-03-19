use chrono::{DateTime, Utc};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlanType {
    Pro,
    Max5,
    Max20,
}

impl PlanType {
    pub fn token_limit(&self) -> u64 {
        match self {
            PlanType::Pro => 19_000,
            PlanType::Max5 => 88_000,
            PlanType::Max20 => 220_000,
        }
    }

    pub fn cost_limit(&self) -> f64 {
        match self {
            PlanType::Pro => 18.0,
            PlanType::Max5 => 35.0,
            PlanType::Max20 => 140.0,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            PlanType::Pro => "Pro",
            PlanType::Max5 => "Max5",
            PlanType::Max20 => "Max20",
        }
    }

    pub fn from_str(s: &str) -> Option<PlanType> {
        match s.to_lowercase().as_str() {
            "pro" => Some(PlanType::Pro),
            "max5" => Some(PlanType::Max5),
            "max20" => Some(PlanType::Max20),
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

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Session {
    pub project_path: String,
    pub folder_name: String,
    pub total_usage: TokenUsage,
    pub total_cost: f64,
    pub last_model: String,
    pub has_thinking: bool,
    pub last_activity: DateTime<Utc>,
    pub is_active: bool,
    pub jsonl_files: Vec<PathBuf>,
}
