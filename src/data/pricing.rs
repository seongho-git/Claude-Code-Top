use super::types::{ModelTier, TokenUsage};

struct PricingTier {
    input_per_m: f64,
    output_per_m: f64,
    cache_create_per_m: f64,
    cache_read_per_m: f64,
}

fn get_pricing(tier: ModelTier) -> PricingTier {
    match tier {
        ModelTier::Opus => PricingTier {
            input_per_m: 15.0,
            output_per_m: 75.0,
            cache_create_per_m: 18.75,
            cache_read_per_m: 1.50,
        },
        ModelTier::Sonnet => PricingTier {
            input_per_m: 3.0,
            output_per_m: 15.0,
            cache_create_per_m: 3.75,
            cache_read_per_m: 0.30,
        },
        ModelTier::Haiku => PricingTier {
            input_per_m: 0.25,
            output_per_m: 1.25,
            cache_create_per_m: 0.30,
            cache_read_per_m: 0.03,
        },
    }
}

pub fn calculate_cost(model_name: &str, usage: &TokenUsage) -> (f64, f64) {
    let tier = ModelTier::from_model_name(model_name);
    let p = get_pricing(tier);
    let m = 1_000_000.0;

    let total_cost = (usage.input_tokens as f64 / m) * p.input_per_m
        + (usage.output_tokens as f64 / m) * p.output_per_m
        + (usage.cache_creation_input_tokens as f64 / m) * p.cache_create_per_m
        + (usage.cache_read_input_tokens as f64 / m) * p.cache_read_per_m;

    let without_cache_cost = ((usage.input_tokens + usage.cache_read_input_tokens) as f64 / m)
        * p.input_per_m
        + (usage.output_tokens as f64 / m) * p.output_per_m;

    let saved = (without_cache_cost - total_cost).max(0.0);

    (total_cost, saved)
}
