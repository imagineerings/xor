use language_model::{LanguageModelProviderId, RateLimiter, RateLimiterConfig};

pub fn default_rate_limiter(provider_id: &LanguageModelProviderId) -> RateLimiter {
    RateLimiter::with_config(default_rate_limit_config(provider_id))
}

pub fn default_rate_limit_config(provider_id: &LanguageModelProviderId) -> RateLimiterConfig {
    match provider_id.to_string().as_str() {
        "anthropic" => RateLimiterConfig::requests_per_minute(4, 50).with_burst_size(8),
        "openai" => RateLimiterConfig::requests_per_minute(4, 60).with_burst_size(10),
        _ => RateLimiterConfig::concurrent(4),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configures_known_provider_limits() {
        let config = default_rate_limit_config(&LanguageModelProviderId::new("openai"));

        assert_eq!(config.max_concurrent_requests, 4);
        assert_eq!(config.requests_per_minute, Some(60));
        assert_eq!(config.burst_size, 10);
    }

    #[test]
    fn defaults_unknown_providers_to_concurrency_limit() {
        let config = default_rate_limit_config(&LanguageModelProviderId::new("custom"));

        assert_eq!(config, RateLimiterConfig::concurrent(4));
    }
}
