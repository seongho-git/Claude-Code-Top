use std::fs;
use std::path::PathBuf;

pub fn app_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude-code-top"))
}

pub fn ensure_app_dir() -> Option<PathBuf> {
    let dir = app_dir()?;
    fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

pub fn config_path() -> Option<PathBuf> {
    app_dir().map(|dir| dir.join("config.json"))
}

pub fn legacy_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".cctop.json"))
}

pub fn usage_cache_path() -> Option<PathBuf> {
    app_dir().map(|dir| dir.join("usage.json"))
}

pub fn legacy_usage_cache_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".cctop-usage.json"))
}
