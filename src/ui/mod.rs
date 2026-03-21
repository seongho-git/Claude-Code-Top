pub mod confirm;
pub mod details;
pub mod header;
pub mod layout;
pub mod theme;
pub mod threads;

/// Shorten a Claude model ID for display (e.g. "claude-opus-4-6-20250101" → "opus-4-6").
pub fn shorten_model(name: &str) -> String {
    if name.is_empty() {
        return "-".to_string();
    }
    let s = name.replace("claude-", "");
    if let Some(pos) = s.rfind('-') {
        let suffix = &s[pos + 1..];
        if suffix.len() == 8 && suffix.chars().all(|c| c.is_ascii_digit()) {
            return s[..pos].to_string();
        }
    }
    s
}

/// Format a token count for compact display (e.g. 1_500_000 → "1.5M").
pub fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// Map raw effort level string to display label.
pub fn effort_display(effort: &str) -> &'static str {
    match effort {
        "max" => "Max",
        "high" => "High",
        "low" => "Low",
        _ => "Auto",
    }
}
