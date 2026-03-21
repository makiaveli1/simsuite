use crate::adapters::curseforge::CurseForgeAdapter;
use crate::adapters::SourceAdapter;

#[test]
fn test_curseforge_adapter_with_api_key() {
    let api_key = "test-api-key-12345";
    let adapter = CurseForgeAdapter::new(Some(api_key.to_string()));
    assert_eq!(adapter.kind(), crate::models::SourceKind::CurseForge);
}

#[test]
fn test_curseforge_adapter_without_api_key() {
    let adapter = CurseForgeAdapter::new(None);
    assert_eq!(adapter.kind(), crate::models::SourceKind::CurseForge);
}

#[test]
fn test_curseforge_adapter_default() {
    let adapter = CurseForgeAdapter::default();
    assert_eq!(adapter.kind(), crate::models::SourceKind::CurseForge);
}
