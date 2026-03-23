use std::fs;
use std::io::{self, Write};

use serde::{Deserialize, Serialize};

use crate::data::types::PlanType;
use crate::paths::{config_path, ensure_app_dir, legacy_config_path};

#[derive(Serialize, Deserialize)]
struct ConfigFile {
    plan: String,
}

pub fn load_plan() -> Option<PlanType> {
    let paths = [config_path(), legacy_config_path()];

    for path in paths.into_iter().flatten() {
        let data = match fs::read_to_string(&path) {
            Ok(data) => data,
            Err(_) => continue,
        };
        let config: ConfigFile = match serde_json::from_str(&data) {
            Ok(config) => config,
            Err(_) => continue,
        };
        if let Some(plan) = PlanType::from_str(&config.plan) {
            return Some(plan);
        }
    }

    None
}

pub fn save_plan(plan: PlanType) {
    if ensure_app_dir().is_some() {
        if let Some(path) = config_path() {
            let config = ConfigFile {
                plan: plan.label().to_lowercase(),
            };
            if let Ok(json) = serde_json::to_string_pretty(&config) {
                let _ = fs::write(path, json);
            }
        }
    }
}

/// Interactive plan selection for first run.
pub fn prompt_plan_selection() -> PlanType {
    println!("\n  cctop — Claude Code Session Monitor\n");
    println!("  Select your Claude plan:\n");
    println!("    1) Pro   — $18/week,  19k output tokens");
    println!("    2) Max5  — $35/week,  88k output tokens");
    println!("    3) Max20 — $140/week, 220k output tokens");
    println!();
    print!("  Enter choice [1/2/3]: ");
    let _ = io::stdout().flush();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        println!("  Defaulting to Max5.");
        return PlanType::Max5;
    }

    match input.trim() {
        "1" => {
            println!("  → Pro selected.\n");
            PlanType::Pro
        }
        "2" => {
            println!("  → Max5 selected.\n");
            PlanType::Max5
        }
        "3" => {
            println!("  → Max20 selected.\n");
            PlanType::Max20
        }
        _ => {
            println!("  Invalid choice. Defaulting to Max5.\n");
            PlanType::Max5
        }
    }
}
