use crate::adapters::{
    apply_custom_headers, detect_access_tier, AdapterError, RemoteSnapshot, SnapshotEvidence,
    SourceAdapter,
};
use crate::error::AppResult;
use crate::models::{SourceBinding, SourceKind};
use crate::services::SharedRateLimiter;
use quick_xml::de::from_str;
use reqwest::blocking::Client;

pub struct FeedAdapter {
    client: Client,
    rate_limiter: SharedRateLimiter,
}

impl FeedAdapter {
    pub fn new(rate_limiter: SharedRateLimiter) -> Self {
        Self {
            client: Client::builder()
                .user_agent("SimSort/1.0")
                .build()
                .unwrap_or_default(),
            rate_limiter,
        }
    }

    fn check_rate_limit(&self, url: &str) -> AppResult<()> {
        if !self.rate_limiter.can_fetch(url) {
            return Err(AdapterError::RateLimited(url.to_string()).into());
        }
        Ok(())
    }

    fn record_request(&self, url: &str) {
        self.rate_limiter.record_fetch(url);
    }

    pub fn parse_feed(&self, content: &str) -> Result<(String, Vec<FeedEntry>), AdapterError> {
        if let Ok(feed) = from_str::<Rss2Feed>(content) {
            let entries: Vec<FeedEntry> = feed
                .channel
                .items
                .into_iter()
                .map(FeedEntry::Rss2)
                .collect();
            return Ok((feed.channel.title, entries));
        }
        if let Ok(feed) = from_str::<AtomFeed>(content) {
            let entries: Vec<FeedEntry> = feed.entries.into_iter().map(FeedEntry::Atom).collect();
            return Ok((feed.title, entries));
        }
        Err(AdapterError::Parse("Unknown feed format".into()))
    }
}

impl SourceAdapter for FeedAdapter {
    fn kind(&self) -> SourceKind {
        SourceKind::Feed
    }

    fn discover_candidates(
        &self,
        _input: &crate::adapters::DiscoverInput,
    ) -> AppResult<Vec<crate::adapters::CandidateSource>> {
        Ok(vec![])
    }

    fn refresh_snapshot(&self, binding: &SourceBinding) -> AppResult<RemoteSnapshot> {
        let url = &binding.source_url;
        self.check_rate_limit(url)?;
        let mut request = self.client.get(&binding.source_url);
        request = apply_custom_headers(request, binding);
        let response = request.send().map_err(AdapterError::Network)?;

        self.record_request(url);
        let etag = response
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        let last_modified = response
            .headers()
            .get("last-modified")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        let body = response.text().map_err(AdapterError::Network)?;

        let (title, entries) = self.parse_feed(&body)?;

        let latest = entries.into_iter().next();

        let (version_text, published_at, release_id, download_url) = if let Some(entry) = latest {
            entry.extract()
        } else {
            (None, None, None, None)
        };

        Ok(RemoteSnapshot {
            binding_id: binding.id.clone(),
            title: Some(title),
            version_text,
            published_at,
            download_url,
            changelog_url: None,
            release_id,
            release_asset_names: vec![],
            image_hashes: vec![],
            etag,
            last_modified,
            evidence: SnapshotEvidence::default(),
            confidence: 0.85,
            raw: serde_json::json!({}),
            access_tier: detect_access_tier(&binding.source_url),
            patron_free_version: None,
            file_fingerprints_json: None,
        })
    }
}

impl Default for FeedAdapter {
    fn default() -> Self {
        Self::new(SharedRateLimiter::default())
    }
}

#[derive(Debug)]
pub enum FeedEntry {
    Rss2(Rss2Item),
    Atom(AtomEntry),
}

impl FeedEntry {
    pub fn title(&self) -> Option<String> {
        match self {
            FeedEntry::Rss2(item) => item.title.clone(),
            FeedEntry::Atom(entry) => entry.title.clone(),
        }
    }

    pub fn guid(&self) -> Option<String> {
        match self {
            FeedEntry::Rss2(item) => item.guid.clone(),
            FeedEntry::Atom(entry) => entry.id.clone(),
        }
    }

    pub fn published(&self) -> Option<String> {
        match self {
            FeedEntry::Rss2(item) => item.pub_date.clone(),
            FeedEntry::Atom(entry) => entry.published.clone().or(entry.updated.clone()),
        }
    }

    pub fn download_url(&self) -> Option<String> {
        match self {
            FeedEntry::Rss2(item) => item.enclosure.as_ref().map(|e| e.url.clone()),
            FeedEntry::Atom(entry) => entry
                .links
                .iter()
                .find(|l| l.rel == "enclosure")
                .map(|l| l.href.clone()),
        }
    }

    fn extract(
        self,
    ) -> (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ) {
        match self {
            FeedEntry::Rss2(item) => (
                item.title,
                item.pub_date,
                item.guid,
                item.enclosure.map(|e| e.url),
            ),
            FeedEntry::Atom(entry) => (
                entry.title,
                entry.published.or(entry.updated),
                entry.id,
                entry
                    .links
                    .iter()
                    .find(|l| l.rel == "enclosure")
                    .map(|l| l.href.clone()),
            ),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct Rss2Feed {
    channel: Rss2Channel,
}

#[derive(Debug, serde::Deserialize)]
pub struct Rss2Channel {
    title: String,
    #[serde(rename = "item", default)]
    items: Vec<Rss2Item>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Rss2Item {
    title: Option<String>,
    #[serde(rename = "pubDate", default)]
    pub_date: Option<String>,
    guid: Option<String>,
    enclosure: Option<Rss2Enclosure>,
    #[serde(default)]
    link: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Rss2Enclosure {
    #[serde(rename = "@url")]
    url: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct AtomFeed {
    title: String,
    #[serde(rename = "entry", default)]
    entries: Vec<AtomEntry>,
}

#[derive(Debug, serde::Deserialize)]
pub struct AtomEntry {
    title: Option<String>,
    id: Option<String>,
    published: Option<String>,
    updated: Option<String>,
    #[serde(rename = "link", default)]
    links: Vec<AtomLink>,
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct AtomLink {
    #[serde(rename = "@href")]
    href: String,
    #[serde(rename = "@rel", default)]
    rel: String,
}

#[cfg(test)]
mod tests {
    use crate::adapters::feed::FeedAdapter;
    use crate::adapters::SourceAdapter;
    use crate::models::{AccessTier, SourceBinding, SourceKind};
    use crate::services::SharedRateLimiter;

    const RSS2_SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Test Feed</title>
    <link>https://example.com/feed</link>
    <description>A test RSS 2.0 feed</description>
    <item>
      <title>Version 2.0 Release</title>
      <link>https://example.com/releases/v2</link>
      <guid>release-2.0</guid>
      <pubDate>Wed, 15 Jan 2025 12:00:00 GMT</pubDate>
      <enclosure url="https://example.com/downloads/v2.zip" length="1024" type="application/zip"/>
    </item>
    <item>
      <title>Version 1.0 Release</title>
      <link>https://example.com/releases/v1</link>
      <guid>release-1.0</guid>
      <pubDate>Tue, 14 Jan 2025 12:00:00 GMT</pubDate>
      <enclosure url="https://example.com/downloads/v1.zip" length="512" type="application/zip"/>
    </item>
  </channel>
</rss>"#;

    const ATOM_SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Test Atom Feed</title>
  <id>urn:uuid:test-feed</id>
  <updated>2025-01-15T12:00:00Z</updated>
  <entry>
    <title>Version 3.0 Release</title>
    <id>urn:uuid:entry-3</id>
    <published>2025-01-15T10:00:00Z</published>
    <updated>2025-01-15T12:00:00Z</updated>
    <link href="https://example.com/releases/v3" rel="alternate"/>
    <link href="https://example.com/downloads/v3.zip" rel="enclosure"/>
    <content>Version 3.0 is here!</content>
  </entry>
  <entry>
    <title>Version 2.0 Release</title>
    <id>urn:uuid:entry-2</id>
    <published>2025-01-10T10:00:00Z</published>
    <updated>2025-01-10T12:00:00Z</updated>
    <link href="https://example.com/releases/v2" rel="alternate"/>
    <content>Version 2.0 is here!</content>
  </entry>
</feed>"#;

    #[test]
    fn test_parse_rss2_feed() {
        let adapter = FeedAdapter::new(SharedRateLimiter::default());
        let result = adapter.parse_feed(RSS2_SAMPLE);
        assert!(result.is_ok(), "Should parse RSS2: {:?}", result.err());

        let (title, entries) = result.expect("Should have feed");
        assert_eq!(title, "Test Feed");
        assert_eq!(entries.len(), 2);

        let first = &entries[0];
        assert_eq!(first.title().as_deref(), Some("Version 2.0 Release"));
        assert_eq!(first.guid().as_deref(), Some("release-2.0"));
        assert_eq!(
            first.download_url().as_deref(),
            Some("https://example.com/downloads/v2.zip")
        );
    }

    #[test]
    fn test_parse_atom_feed() {
        let adapter = FeedAdapter::new(SharedRateLimiter::default());
        let result = adapter.parse_feed(ATOM_SAMPLE);
        assert!(result.is_ok(), "Should parse Atom: {:?}", result.err());

        let (title, entries) = result.expect("Should have feed");
        assert_eq!(title, "Test Atom Feed");
        assert_eq!(entries.len(), 2);

        let first = &entries[0];
        assert_eq!(first.title().as_deref(), Some("Version 3.0 Release"));
        assert_eq!(first.guid().as_deref(), Some("urn:uuid:entry-3"));
        assert_eq!(
            first.download_url().as_deref(),
            Some("https://example.com/downloads/v3.zip")
        );
    }

    #[test]
    fn test_detect_new_entry() {
        let adapter = FeedAdapter::new(SharedRateLimiter::default());
        let result = adapter.parse_feed(RSS2_SAMPLE).expect("Should parse");
        let (_title, entries) = result;

        let first_guid = entries[0].guid();
        assert_eq!(first_guid.as_deref(), Some("release-2.0"));
    }

    #[test]
    fn test_feed_adapter_kind() {
        let adapter = FeedAdapter::new(SharedRateLimiter::default());
        assert_eq!(adapter.kind(), SourceKind::Feed);
    }

    #[test]
    #[ignore] // Requires actual HTTP server - tested manually with real feeds
    fn test_refresh_snapshot_extracts_latest() {
        let adapter = FeedAdapter::new(SharedRateLimiter::default());

        let binding = SourceBinding {
            id: "test-binding-1".to_string(),
            local_mod_id: "test-mod-1".to_string(),
            source_kind: SourceKind::Feed,
            source_url: "https://example.com/feed.xml".to_string(),
            provider_mod_id: None,
            provider_file_id: None,
            provider_repo: None,
            bind_method: "manual".to_string(),
            is_primary: true,
            custom_headers_json: None,
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
        };

        let result = adapter.refresh_snapshot(&binding);
        assert!(result.is_ok(), "Should succeed: {:?}", result.err());

        let snapshot = result.expect("Should have result");
        assert_eq!(snapshot.binding_id, "test-binding-1");
        assert!(snapshot.version_text.is_some(), "Should have version text");
    }
}
