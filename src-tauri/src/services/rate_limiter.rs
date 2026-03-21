use std::collections::HashMap;
use std::sync::{Arc, RwLock};
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

struct RetryState {
    attempts: u32,
    last_attempt: Instant,
    base_delay: Duration,
}

#[derive(Clone)]
pub struct SharedRateLimiter {
    inner: Arc<RwLock<DomainRateLimiter>>,
    retry_state: Arc<RwLock<HashMap<String, RetryState>>>,
}

impl SharedRateLimiter {
    pub fn new(min_interval: Duration) -> Self {
        Self {
            inner: Arc::new(RwLock::new(DomainRateLimiter::new(min_interval))),
            retry_state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn can_fetch(&self, url: &str) -> bool {
        let domain = self.extract_domain(url);
        if let Some(ref d) = domain {
            let backoff = self.wait_time_for_retry(d);
            if backoff > Duration::ZERO {
                return false;
            }
        }
        self.inner.read().unwrap().can_fetch(url)
    }

    pub fn record_fetch(&self, url: &str) {
        self.inner.write().unwrap().record_fetch(url);
        if let Some(domain) = self.extract_domain(url) {
            self.reset_retry_state(&domain);
        }
    }

    pub fn wait_time(&self, url: &str) -> Duration {
        self.inner.read().unwrap().wait_time(url)
    }

    pub fn record_rate_limit(&self, domain: &str) {
        let mut state = self.retry_state.write().unwrap();
        if let Some(retry) = state.get_mut(domain) {
            retry.attempts += 1;
            retry.last_attempt = Instant::now();
        } else {
            state.insert(
                domain.to_string(),
                RetryState {
                    attempts: 1,
                    last_attempt: Instant::now(),
                    base_delay: Duration::from_secs(1),
                },
            );
        }
    }

    pub fn wait_time_for_retry(&self, domain: &str) -> Duration {
        let state = self.retry_state.read().unwrap();
        if let Some(retry) = state.get(domain) {
            let delay = retry.base_delay * 2u32.pow(retry.attempts.min(4));
            delay.min(Duration::from_secs(300))
        } else {
            Duration::ZERO
        }
    }

    pub fn reset_retry_state(&self, domain: &str) {
        let mut state = self.retry_state.write().unwrap();
        state.remove(domain);
    }

    fn extract_domain(&self, url: &str) -> Option<String> {
        Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(|s| s.to_string()))
    }
}

impl Default for SharedRateLimiter {
    fn default() -> Self {
        Self::new(Duration::from_secs(60))
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

    #[test]
    fn test_shared_rate_limiter_clone_shares_state() {
        let limiter = SharedRateLimiter::new(Duration::from_secs(60));
        let cloned = limiter.clone();

        assert!(limiter.can_fetch("https://api.curseforge.com/test"));
        limiter.record_fetch("https://api.curseforge.com/test");
        assert!(!limiter.can_fetch("https://api.curseforge.com/test"));

        assert!(!cloned.can_fetch("https://api.curseforge.com/test"));
    }

    #[test]
    fn test_shared_rate_limiter_record_fetch() {
        let limiter = SharedRateLimiter::new(Duration::from_secs(60));
        assert!(limiter.can_fetch("https://api.curseforge.com/test"));

        limiter.record_fetch("https://api.curseforge.com/test");
        assert!(!limiter.can_fetch("https://api.curseforge.com/test"));
    }

    #[test]
    fn test_shared_rate_limiter_wait_time() {
        let limiter = SharedRateLimiter::new(Duration::from_secs(60));
        assert_eq!(
            limiter.wait_time("https://api.curseforge.com/test"),
            Duration::ZERO
        );

        limiter.record_fetch("https://api.curseforge.com/test");
        let wait = limiter.wait_time("https://api.curseforge.com/test");
        assert!(wait > Duration::ZERO);
        assert!(wait <= Duration::from_secs(60));
    }

    #[test]
    fn test_shared_rate_limiter_default() {
        let limiter = SharedRateLimiter::default();
        assert!(limiter.can_fetch("https://api.example.com/test"));
    }

    #[test]
    fn test_shared_rate_limiter_different_domains_independent() {
        let limiter = SharedRateLimiter::new(Duration::from_secs(60));
        limiter.record_fetch("https://api.curseforge.com/test");
        assert!(!limiter.can_fetch("https://api.curseforge.com/test"));
        assert!(limiter.can_fetch("https://api.github.com/test"));
    }
}
