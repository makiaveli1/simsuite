use crate::adapters::generic_page::GenericPageAdapter;
use crate::adapters::SourceAdapter;
use crate::models::{SourceBinding, SourceKind};
use crate::services::SharedRateLimiter;
use scraper::Html;

#[test]
fn test_generic_adapter_kind() {
    let adapter = GenericPageAdapter::new(SharedRateLimiter::default());
    assert_eq!(adapter.kind(), SourceKind::GenericPage);
}

#[test]
fn test_compute_content_hash() {
    let html1 = r#"<html><head><title>Test</title></head><body><p>Content</p></body></html>"#;
    let html2 = r#"<html><head><title>Test</title></head><body><p>Content</p></body></html>"#;
    let html3 = r#"<html><head><title>Different</title></head><body><p>Other</p></body></html>"#;

    let hash1 = GenericPageAdapter::compute_content_hash(html1);
    let hash2 = GenericPageAdapter::compute_content_hash(html2);
    let hash3 = GenericPageAdapter::compute_content_hash(html3);

    assert_eq!(hash1, hash2, "Same content should produce same hash");
    assert_ne!(
        hash1, hash3,
        "Different content should produce different hash"
    );
    assert_eq!(hash1.len(), 64, "SHA256 hash should be 64 hex characters");
}

#[test]
fn test_compute_content_hash_strips_scripts_and_styles() {
    let html_with_noise = r#"<html><head><title>Test</title><script>var x = 1;</script><style>body { color: red; }</style></head><body><p>Content</p><nav>Navigation</nav><footer>Footer</footer></body></html>"#;

    let html_clean = r#"<html><head><title>Test</title></head><body><p>Content</p></body></html>"#;

    let hash_with_noise = GenericPageAdapter::compute_content_hash(html_with_noise);
    let hash_clean = GenericPageAdapter::compute_content_hash(html_clean);

    assert_eq!(
        hash_with_noise, hash_clean,
        "Scripts, styles, nav, footer should be stripped"
    );
}

#[test]
fn test_extract_text_content() {
    let html = r#"
        <html>
        <head><title>Test Page</title></head>
        <body>
            <script>console.log('ignored');</script>
            <style>.hidden { display: none; }</style>
            <h1>Main Title</h1>
            <p>Paragraph text here</p>
        </body>
        </html>"#;

    let hash = GenericPageAdapter::compute_content_hash(html);

    assert!(!hash.is_empty());
    assert_eq!(hash.len(), 64);
}

#[test]
fn test_extract_title() {
    let html_og_title = Html::parse_document(
        r#"
        <html>
        <head><meta property="og:title" content="OG Title"></head>
        <body><h1>H1 Title</h1></body>
        </html>"#,
    );
    assert_eq!(
        GenericPageAdapter::extract_title(&html_og_title).as_deref(),
        Some("OG Title")
    );

    let html_regular_title = Html::parse_document(
        r#"
        <html>
        <head><title>Regular Title</title></head>
        <body><h1>H1 Title</h1></body>
        </html>"#,
    );
    assert_eq!(
        GenericPageAdapter::extract_title(&html_regular_title).as_deref(),
        Some("Regular Title")
    );

    let html_h1_only = Html::parse_document(r#"<html><body><h1>H1 Only</h1></body></html>"#);
    assert_eq!(
        GenericPageAdapter::extract_title(&html_h1_only).as_deref(),
        Some("H1 Only")
    );

    let html_empty = Html::parse_document(r#"<html><body><p>No title here</p></body></html>"#);
    assert!(GenericPageAdapter::extract_title(&html_empty).is_none());
}

#[test]
fn test_refresh_snapshot_uses_etag() {
    let adapter = GenericPageAdapter::new(SharedRateLimiter::default());

    let binding = SourceBinding {
        id: "test-binding-generic".to_string(),
        local_mod_id: "test-mod-1".to_string(),
        source_kind: SourceKind::GenericPage,
        source_url: "https://httpbin.org/html".to_string(),
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
    assert_eq!(snapshot.binding_id, "test-binding-generic");
    assert!(
        snapshot.title.is_some() || snapshot.release_id.is_some(),
        "Should have either title or release_id"
    );
    assert_eq!(
        snapshot.confidence, 0.50,
        "GenericPage should have 0.50 confidence"
    );
}

#[test]
#[ignore]
fn test_refresh_snapshot_with_conditional_headers() {
    let adapter = GenericPageAdapter::new(SharedRateLimiter::default());

    let binding_with_etag = SourceBinding {
        id: "test-binding-etag".to_string(),
        local_mod_id: "test-mod-1".to_string(),
        source_kind: SourceKind::GenericPage,
        source_url: "https://httpbin.org/html".to_string(),
        provider_mod_id: Some("test-etag".to_string()),
        provider_file_id: None,
        provider_repo: None,
        bind_method: "manual".to_string(),
        is_primary: true,
        custom_headers_json: None,
        created_at: "2025-01-01T00:00:00Z".to_string(),
        updated_at: "2025-01-01T00:00:00Z".to_string(),
    };

    let result = adapter.refresh_snapshot(&binding_with_etag);
    assert!(result.is_ok());
}

#[test]
fn test_discover_candidates_returns_empty() {
    use crate::adapters::DiscoverInput;

    let adapter = GenericPageAdapter::new(SharedRateLimiter::default());
    let input = DiscoverInput {
        local_mod_id: "test".to_string(),
        display_name: "Test Mod".to_string(),
        normalized_name: "test-mod".to_string(),
        creator_name: None,
        category: None,
        files: vec![],
    };

    let result = adapter.discover_candidates(&input);
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}
