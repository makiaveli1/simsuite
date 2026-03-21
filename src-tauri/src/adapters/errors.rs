use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum AdapterError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("Not supported: {0}")]
    NotSupported(String),
    #[error("Rate limited for domain: {0}")]
    RateLimited(String),
}

impl From<AdapterError> for crate::error::AppError {
    fn from(value: AdapterError) -> Self {
        match value {
            AdapterError::Network(e) => crate::error::AppError::Http(e),
            AdapterError::Parse(s) => crate::error::AppError::Message(s),
            AdapterError::Api(s) => crate::error::AppError::Message(s),
            AdapterError::NotSupported(s) => crate::error::AppError::Message(s),
            AdapterError::RateLimited(domain) => {
                crate::error::AppError::Message(format!("Rate limited: {}", domain))
            }
        }
    }
}
