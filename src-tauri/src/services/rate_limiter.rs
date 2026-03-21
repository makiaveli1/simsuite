use std::collections::HashMap;
use std::time::{Duration, Instant};
use url::Url;

#[derive(Clone, Debug)]
pub struct DomainRateLimiter {
    domains: HashMap<String, Instant>,
    min_interval: Duration,
}

impl DomainRateLimiter {
    /// Creates a new rate limiter with the specified minimum interval between requests.
    pub fn new(min_interval: Duration) -> Self {
        Self {
            domains: HashMap::new(),
            min_interval,
        }
    }

    /// Returns true if a request to the given URL can be made now.
    /// Takes into account the minimum interval since the last request to the same domain.
    pub fn can_fetch(&self, url: &str) -> bool {
        let domain = self.extract_domain(url);
        if let Some(last_fetch) = self.domains.get(&domain) {
            last_fetch.elapsed() >= self.min_interval
        } else {
            true
        }
    }

    /// Records that a request was made to the given URL.
    pub fn record_fetch(&mut self, url: &str) {
        let domain = self.extract_domain(url);
        self.domains.insert(domain, Instant::now());
    }

    /// Returns the duration to wait before the next request to the given URL can be made.
    pub fn wait_time(&self, url: &str) -> Duration {
        let domain = self.extract_domain(url);
        if let Some(last_fetch) = self.domains.get(&domain) {
            let elapsed = last_fetch.elapsed();
            if elapsed >= self.min_interval {
                Duration::ZERO
            } else {
                self.min_interval - elapsed
            }
        } else {
            Duration::ZERO
        }
    }

    fn extract_domain(&self, url: &str) -> String {
        Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(|s| s.to_string()))
            .unwrap_or_else(|| {
                tracing::warn!("Failed to extract domain from URL: {}", url);
                "unknown".to_string()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_fetch_first_request() {
        let limiter = DomainRateLimiter::new(Duration::from_secs(60));
        assert!(limiter.can_fetch("https://api.curseforge.com/test"));
    }

    #[test]
    fn test_cannot_fetch_within_interval() {
        let mut limiter = DomainRateLimiter::new(Duration::from_secs(60));
        limiter.record_fetch("https://api.curseforge.com/test");
        assert!(!limiter.can_fetch("https://api.curseforge.com/test"));
    }

    #[test]
    fn test_different_domains_independent() {
        let mut limiter = DomainRateLimiter::new(Duration::from_secs(60));
        limiter.record_fetch("https://api.curseforge.com/test");
        assert!(!limiter.can_fetch("https://api.curseforge.com/test"));
        assert!(limiter.can_fetch("https://api.github.com/test"));
    }
}
