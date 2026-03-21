pub struct RateLimiter;

impl RateLimiter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}
