use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom},
    path::Path,
};

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};
use flate2::read::ZlibDecoder;
use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage, ImageEncoder};
use zip::ZipArchive;

use crate::{
    core::filename_parser::detect_creator_hint,
    core::special_mod_versions::extract_version_candidates_with_scores,
    error::{AppError, AppResult},
    models::{FileInsights, VersionSignal},
    seed::SeedPack,
};

const DBPF_HEADER_SIZE: usize = 68;
const MAX_RESOURCE_BYTES: usize = 2 * 1024 * 1024;
const MAX_SCRIPT_ENTRIES: usize = 256;
const MAX_SCRIPT_HINT_ENTRIES: usize = 16;
const MAX_SCRIPT_HINT_BYTES: u64 = 128 * 1024;
const MAX_DISPLAY_VALUES: usize = 8;
const MAX_CREATOR_HINTS: usize = 4;
const MAX_VERSION_SIGNALS: usize = 16;
const MIN_FALLBACK_CREATOR_HINT_LEN: usize = 3;
const STRING_SUPPORT_HINT_TOKENS: &[&str] = &[
    "string",
    "strings",
    "translation",
    "translations",
    "translate",
    "localization",
    "localized",
    "locale",
    "thai",
    "spanish",
    "french",
    "german",
    "italian",
    "polish",
    "russian",
    "japanese",
    "korean",
    "chinese",
    "portuguese",
    "brazilian",
];
const GAMEPLAY_CONTEXT_HINT_TOKENS: &[&str] = &[
    "addon",
    "addons",
    "friendship",
    "greeting",
    "greetings",
    "integration",
    "lag",
    "lagfix",
    "mainmod",
    "module",
    "modules",
    "motive",
    "motives",
    "phone",
    "uicheats",
];
const TS4SCRIPT_NOISE_STEMS: &[&str] = &[
    "__init__",
    "__main__",
    "_do_not_unzip_",
    "readme",
    "changelog",
];

const RESOURCE_NAME_MAP: u32 = 0x0166_038c;
const RESOURCE_STRING_TABLE: u32 = 0x2205_57da;
const RESOURCE_CAS_PART: u32 = 0x034a_eecb;
const RESOURCE_SKINTONE: u32 = 0x0354_796a;
const RESOURCE_CATALOG: u32 = 0x319e_4f1d;
const RESOURCE_DEFINITION: u32 = 0xc0db_5ae7;
const RESOURCE_HOTSPOT: u32 = 0x8b18_ff6e;
const RESOURCE_SCRIPT: u32 = 0x073f_aa07;
const RESOURCE_THUM: u32 = 0x3C1A_F1F2;
const MAX_THUMBNAIL_BYTES: usize = 512 * 1024;
const BUILD_SURFACE_RESOURCE_TYPES: &[u32] = &[
    0x01d0_e75d,
    0xb4f7_62c9,
    0xd5f0_f921,
    0xebcb_b16c,
    0xf1ed_bd86,
];
const BUILD_STRUCTURE_RESOURCE_TYPES: &[u32] = &[
    0x0201_9972,
    0x2fae_983e,
    0x76bc_f80c,
    0x0418_fe2a,
    0xd382_bf57,
    0x1c1c_f1f7,
    0x3f0c_529a,
    0x9a20_cd1c,
    0xa057_811c,
    0x84c2_3219,
];
const LEAN_CAS_APPEARANCE_RESOURCE_TYPES: &[u32] = &[0x015a_1849, 0xac16_fbec];
const GAMEPLAY_RESOURCE_WEIGHTS: &[(u32, usize)] = &[
    (0x0c77_2e27, 3),
    (0x545a_c67a, 2),
    (0x6017_e896, 2),
    (0x03b3_3ddf, 1),
    (0x03e9_d964, 1),
    (0x0069_453e, 1),
    (0x0e4d_15fb, 1),
    (0x2553_f435, 1),
    (0x28b6_4675, 1),
    (0x2c70_adf8, 1),
    (0x339b_c5bd, 1),
    (0x5107_7643, 1),
    (0x5806_f5ba, 1),
    (0x5b02_819e, 1),
    (0x6e0d_da9f, 1),
    (0x7df2_169c, 1),
    (0x7fb6_ad8a, 1),
    (0xb61d_e6b4, 1),
    (0xcb5f_ddc7, 1),
    (0xe882_d22f, 2),
    (0xec6a_8fc6, 1),
];

#[derive(Debug, Clone, Default)]
pub struct InspectionOutcome {
    pub insights: FileInsights,
    pub creator_hint: Option<String>,
    pub kind_hint: Option<String>,
    pub subtype_hint: Option<String>,
    pub confidence_boost: f64,
    pub kind_confidence_floor: f64,
}

#[derive(Debug, Clone, Copy)]
struct DbpfHeader {
    record_count: u32,
    index_size: u32,
    index_offset: u32,
}

#[derive(Debug, Clone, Copy)]
struct DbpfRecord {
    resource_type: u32,
    offset: u32,
    packed_size: u32,
    mem_size: u32,
    compressed: u16,
}

impl DbpfRecord {
    fn is_compressed(&self) -> bool {
        self.compressed != 0 || (self.mem_size > 0 && self.mem_size != self.packed_size)
    }
}

pub fn inspect_file(
    path: &Path,
    extension: &str,
    seed_pack: &SeedPack,
) -> AppResult<InspectionOutcome> {
    match extension {
        ".ts4script" => inspect_ts4script(path, seed_pack),
        ".package" => inspect_package(path, seed_pack),
        _ => Ok(InspectionOutcome::default()),
    }
}

fn inspect_ts4script(path: &Path, seed_pack: &SeedPack) -> AppResult<InspectionOutcome> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)
        .map_err(|error| AppError::Message(format!("Invalid ts4script archive: {error}")))?;

    let mut namespaces = BTreeSet::new();
    let mut version_stems = BTreeSet::new();
    let mut identity_stems = BTreeSet::new();
    let mut payload_values = Vec::new();
    let mut archive_paths = Vec::new();
    let mut payload_reads = 0usize;

    for index in 0..archive.len().min(MAX_SCRIPT_ENTRIES) {
        let entry = archive.by_index(index).map_err(|error| {
            AppError::Message(format!("Unable to inspect ts4script entry: {error}"))
        })?;
        if entry.name().ends_with('/') {
            continue;
        }

        let entry_name = entry.name().replace('\\', "/");
        archive_paths.push(entry_name.clone());
        for namespace in ts4script_namespace_candidates(&entry_name) {
            namespaces.insert(namespace);
        }

        if let Some(filename) = entry_name.rsplit('/').next() {
            let stem = filename.split('.').next().unwrap_or(filename).trim();
            if !stem.is_empty() {
                version_stems.insert(stem.to_owned());
                if let Some(identity_stem) = clean_ts4script_identity_stem(stem) {
                    identity_stems.insert(identity_stem);
                }
            }
        }

        let entry_name_lower = entry_name.to_ascii_lowercase();
        if payload_reads < MAX_SCRIPT_HINT_ENTRIES
            && should_read_ts4script_payload_for_hints(&entry_name_lower)
        {
            let mut bytes = Vec::new();
            entry
                .take(MAX_SCRIPT_HINT_BYTES)
                .read_to_end(&mut bytes)
                .map_err(AppError::from)?;
            if !bytes.is_empty() {
                payload_values.push((
                    entry_name.clone(),
                    String::from_utf8_lossy(&bytes).to_string(),
                ));
                payload_reads += 1;
            }
        }
    }

    let payload_identity_values = extract_ts4script_payload_identity_values(
        payload_values
            .iter()
            .map(|(entry_name, payload)| (entry_name.as_str(), payload.as_str())),
    );
    let raw_identity_values = namespaces
        .iter()
        .chain(identity_stems.iter())
        .chain(payload_identity_values.iter())
        .map(String::as_str)
        .collect::<Vec<_>>();
    let creator_hints = collect_creator_hints(raw_identity_values.iter().copied(), seed_pack);
    let version_signals = collect_ts4script_version_signals(
        path,
        version_stems.iter().map(String::as_str),
        archive_paths.iter().map(String::as_str),
        payload_values
            .iter()
            .map(|(entry_name, payload)| (entry_name.as_str(), payload.as_str())),
    );
    let version_hints = derive_version_hints(&version_signals);
    let family_hints = collect_family_hints(raw_identity_values.iter().copied());

    let mut script_namespaces = namespaces
        .into_iter()
        .take(MAX_DISPLAY_VALUES)
        .collect::<Vec<_>>();
    if script_namespaces.is_empty() {
        script_namespaces = identity_stems
            .iter()
            .take(MAX_DISPLAY_VALUES)
            .cloned()
            .collect();
    }

    let primary_creator = creator_hints.first().cloned();
    let mut embedded_names = identity_stems.into_iter().collect::<Vec<_>>();
    embedded_names.extend(payload_identity_values);
    embedded_names = unique_display_values(embedded_names);

    Ok(InspectionOutcome {
        insights: FileInsights {
            format: Some("ts4script-zip".to_owned()),
            resource_summary: build_ts4script_resource_summary(
                &archive_paths,
                script_namespaces.len(),
            ),
            script_namespaces,
            embedded_names,
            creator_hints: creator_hints.clone(),
            version_hints,
            version_signals,
            family_hints,
            thumbnail_preview: None,
        },
        creator_hint: primary_creator,
        kind_hint: Some("ScriptMods".to_owned()),
        subtype_hint: Some("Utilities".to_owned()),
        confidence_boost: 0.14,
        kind_confidence_floor: 0.7,
    })
}

fn ts4script_namespace_candidates(entry_name: &str) -> Vec<String> {
    let segments = entry_name
        .split('/')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if segments.len() <= 1 {
        return Vec::new();
    }
    let directory_count = segments.len() - 1;

    segments
        .into_iter()
        .take(directory_count)
        .take(2)
        .filter_map(clean_ts4script_identity_stem)
        .collect()
}

fn clean_ts4script_identity_stem(value: &str) -> Option<String> {
    let cleaned = value.trim();
    if cleaned.is_empty() {
        return None;
    }

    let stem = cleaned.split('.').next().unwrap_or(cleaned).trim();
    if stem.is_empty() {
        return None;
    }

    let lowered = stem.to_ascii_lowercase();
    if TS4SCRIPT_NOISE_STEMS.contains(&lowered.as_str()) {
        return None;
    }

    Some(stem.to_owned())
}

fn inspect_package(path: &Path, seed_pack: &SeedPack) -> AppResult<InspectionOutcome> {
    let mut file = File::open(path)?;
    let header = match parse_dbpf_header(&mut file) {
        Ok(header) => header,
        Err(_) => return Ok(InspectionOutcome::default()),
    };
    let records = parse_dbpf_records(&mut file, header)?;

    let mut type_counts = BTreeMap::new();
    for record in &records {
        *type_counts.entry(record.resource_type).or_insert(0_usize) += 1;
    }

    let stbl_map = collect_string_table_entries(&mut file, &records)?;
    let catalog_names = collect_catalog_names(&mut file, &records, &stbl_map)?;
    let cas_names = collect_cas_part_names(&mut file, &records)?;
    let name_map_values = collect_name_map_values(&mut file, &records)?;

    let mut embedded_names = Vec::new();
    embedded_names.extend(catalog_names);
    embedded_names.extend(cas_names);
    embedded_names.extend(name_map_values);
    embedded_names = unique_display_values(embedded_names);

    let creator_hints = collect_creator_hints(embedded_names.iter().map(String::as_str), seed_pack);
    let resource_summary = build_resource_summary(&type_counts, &records);
    let version_signals = collect_package_version_signals(
        path,
        embedded_names.iter().map(String::as_str),
        resource_summary.iter().map(String::as_str),
    );
    let version_hints = derive_version_hints(&version_signals);
    let family_hints = collect_family_hints(
        embedded_names
            .iter()
            .map(String::as_str)
            .chain(creator_hints.iter().map(String::as_str)),
    );
    let resource_kind = infer_kind_from_package_signals(path, &type_counts);
    let thumbnail_preview = extract_thumbnail_preview(&mut file, &records);

    Ok(InspectionOutcome {
        insights: FileInsights {
            format: Some("dbpf-package".to_owned()),
            resource_summary,
            script_namespaces: Vec::new(),
            embedded_names,
            creator_hints: creator_hints.clone(),
            version_hints,
            version_signals,
            family_hints,
            thumbnail_preview,
        },
        creator_hint: creator_hints.first().cloned(),
        kind_hint: resource_kind.kind_hint,
        subtype_hint: resource_kind.subtype_hint,
        confidence_boost: 0.12,
        kind_confidence_floor: resource_kind.confidence_floor,
    })
}
// ─── Thumbnail Extraction ───────────────────────────────────────────────────────

/// Returns base64-encoded PNG of the first THUM resource found, or None.
fn extract_thumbnail_preview(file: &mut File, records: &[DbpfRecord]) -> Option<String> {
    let thum_records: Vec<_> = records
        .iter()
        .filter(|r| r.resource_type == RESOURCE_THUM)
        .collect();

    for record in thum_records {
        if let Ok(raw) = read_record_bytes(file, record) {
            if raw.len() < 24 {
                continue;
            }
            if let Some(png_b64) = decode_thum_to_png(&raw) {
                return Some(png_b64);
            }
        }
    }
    None
}

/// Decode raw THUM bytes (raw BMP DIB data, no file header) to base64 PNG.
fn decode_thum_to_png(raw: &[u8]) -> Option<String> {
    if raw.len() < 24 {
        return None;
    }

    let width  = u32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]);
    let height = u32::from_le_bytes([raw[8], raw[9], raw[10], raw[11]]);

    if width == 0 || height == 0 || width > 1024 || height > 1024 {
        return None;
    }

    // AFLP alpha chunk: magic at byte 24 == 0x41464C41
    let has_alpha = raw.len() >= 32
        && u32::from_le_bytes([raw[24], raw[25], raw[26], raw[27]]) == 0x41464C41;

    // BMP height field can be negative (top-down). Compute absolute value.
    // Bottom 31 bits give the magnitude; negate as two's complement.
    let h_abs = if height & 0x8000_0000 != 0 {
        0u32.wrapping_sub(height)
    } else {
        height
    };
    let bpp   = if has_alpha { 32 } else { 24 };
    let row_stride = ((width as usize * (bpp / 8) + 3) / 4) * 4;
    let pixel_start = 24_usize;

    // Build a valid BMP: 14-byte file header + 40-byte DIB header + pixel rows
    let data_offset: u32 = 14 + 40;
    let image_size = (row_stride * h_abs as usize) as u32;
    let file_size  = data_offset + image_size;

    let mut bmp = Vec::with_capacity(14 + 40 + (raw.len().saturating_sub(pixel_start)));
    bmp.extend_from_slice(&[0x42, 0x4D]); // BM
    bmp.extend_from_slice(&file_size.to_le_bytes());
    bmp.extend_from_slice(&[0u8; 4]);
    bmp.extend_from_slice(&data_offset.to_le_bytes());
    // DIB header
    bmp.extend_from_slice(&40u32.to_le_bytes());
    bmp.extend_from_slice(&width.to_le_bytes());
    bmp.extend_from_slice(&height.to_le_bytes());
    bmp.extend_from_slice(&1u16.to_le_bytes());
    bmp.extend_from_slice(&(bpp as u16).to_le_bytes());
    bmp.extend_from_slice(&[0u8; 20]); // BI_RGB, rest zero
    if raw.len() > pixel_start {
        bmp.extend_from_slice(&raw[pixel_start..]);
    }

    let img = image::load(Cursor::new(&bmp), image::ImageFormat::Bmp).ok()?;
    let mut rgba = img.to_rgba8();

    if has_alpha && raw.len() >= 32 {
        let alpha_len = u32::from_le_bytes([raw[28], raw[29], raw[30], raw[31]]) as usize;
        if raw.len() >= 32 + alpha_len {
            apply_bgra_alpha(&mut rgba, &raw[32..32 + alpha_len], width, h_abs);
        }
    }

    // Encode as PNG
    let mut png_bytes = Vec::new();
    DynamicImage::ImageRgba8(rgba)
        .write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .ok()?;

    Some(BASE64_STANDARD.encode(&png_bytes))
}

/// Apply a BGRA alpha-channel overlay to an RGBA image.
fn apply_bgra_alpha(img: &mut RgbaImage, alpha_data: &[u8], w: u32, h: u32) {
    let expected = (w * h * 4) as usize;
    if alpha_data.len() < expected {
        return;
    }
    for y in 0..h {
        for x in 0..w {
            let i = (y as usize * w as usize + x as usize) * 4;
            let a = alpha_data[i + 3];
            if a < 255 {
                img.get_pixel_mut(x, y).0[3] = a;
            }
        }
    }
}



fn collect_ts4script_version_signals<'a>(
    path: &Path,
    embedded_names: impl IntoIterator<Item = &'a str>,
    archive_paths: impl IntoIterator<Item = &'a str>,
    payload_values: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Vec<VersionSignal> {
    let mut signals = Vec::new();

    if let Some(filename) = path.file_name().and_then(|value| value.to_str()) {
        push_version_signals(
            &mut signals,
            filename,
            "filename",
            Some(path.to_string_lossy().as_ref()),
            Some("file name"),
            0.66,
        );
    }

    for embedded_name in embedded_names {
        push_version_signals(
            &mut signals,
            embedded_name,
            "embedded_name",
            None,
            Some("embedded name"),
            0.54,
        );
    }

    for archive_path in archive_paths {
        push_version_signals(
            &mut signals,
            archive_path,
            "archive_path",
            Some(archive_path),
            Some("archive entry path"),
            0.58,
        );
    }

    for (entry_name, payload) in payload_values {
        push_version_signals(
            &mut signals,
            payload,
            "payload",
            Some(entry_name),
            Some("readable archive payload"),
            0.88,
        );
    }

    sort_and_limit_signals(signals)
}

fn collect_package_version_signals<'a>(
    path: &Path,
    embedded_names: impl IntoIterator<Item = &'a str>,
    resource_summary: impl IntoIterator<Item = &'a str>,
) -> Vec<VersionSignal> {
    let mut signals = Vec::new();

    if let Some(filename) = path.file_name().and_then(|value| value.to_str()) {
        push_version_signals(
            &mut signals,
            filename,
            "filename",
            Some(path.to_string_lossy().as_ref()),
            Some("file name"),
            0.64,
        );
    }

    for embedded_name in embedded_names {
        push_version_signals(
            &mut signals,
            embedded_name,
            "embedded_name",
            None,
            Some("embedded resource name"),
            0.74,
        );
    }

    for summary in resource_summary {
        push_version_signals(
            &mut signals,
            summary,
            "resource_summary",
            None,
            Some("resource summary"),
            0.42,
        );
    }

    sort_and_limit_signals(signals)
}

fn push_version_signals(
    signals: &mut Vec<VersionSignal>,
    value: &str,
    source_kind: &str,
    source_path: Option<&str>,
    matched_by: Option<&str>,
    base_confidence: f64,
) {
    for candidate in extract_version_candidates_with_scores(value) {
        if signals.iter().any(|existing| {
            existing.normalized_value == candidate.normalized
                && existing.source_kind == source_kind
                && existing.source_path.as_deref() == source_path
        }) {
            continue;
        }

        signals.push(VersionSignal {
            raw_value: candidate.raw_value.clone(),
            normalized_value: candidate.normalized,
            source_kind: source_kind.to_owned(),
            source_path: source_path.map(|path| path.to_owned()),
            matched_by: matched_by.map(|value| value.to_owned()),
            confidence: version_signal_confidence(base_confidence, candidate.score),
        });
    }
}

fn version_signal_confidence(base_confidence: f64, score: i64) -> f64 {
    (base_confidence + (score as f64 / 120.0)).clamp(0.0, 0.99)
}

fn sort_and_limit_signals(mut signals: Vec<VersionSignal>) -> Vec<VersionSignal> {
    signals.sort_by(|left, right| {
        right
            .confidence
            .partial_cmp(&left.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.normalized_value.cmp(&right.normalized_value))
    });
    signals.truncate(MAX_VERSION_SIGNALS);
    signals
}

fn derive_version_hints(signals: &[VersionSignal]) -> Vec<String> {
    let mut hints = Vec::new();
    for signal in signals {
        if hints.contains(&signal.normalized_value) {
            continue;
        }
        hints.push(signal.normalized_value.clone());
        if hints.len() >= MAX_DISPLAY_VALUES {
            break;
        }
    }
    hints
}

fn should_read_ts4script_payload_for_hints(entry_name_lower: &str) -> bool {
    if entry_name_lower.contains("game_version") {
        return false;
    }

    entry_name_lower.contains("version")
        || entry_name_lower.ends_with("modfilemanifest.yml")
        || entry_name_lower.ends_with("modfilemanifest.yaml")
        || entry_name_lower.ends_with("modfilemanifest.json")
        || entry_name_lower.ends_with("manifest.yml")
        || entry_name_lower.ends_with("manifest.yaml")
        || entry_name_lower.ends_with("manifest.json")
        || entry_name_lower.ends_with("readme.txt")
        || entry_name_lower.ends_with("readme.md")
        || entry_name_lower.ends_with("changelog.txt")
        || entry_name_lower.ends_with("changelog.md")
}

fn extract_ts4script_payload_identity_values<'a>(
    payload_values: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Vec<String> {
    let mut values = Vec::new();

    for (entry_name, payload) in payload_values {
        let entry_name_lower = entry_name.to_ascii_lowercase();
        if !entry_name_lower.contains("manifest") {
            continue;
        }

        values.extend(extract_ts4script_json_identity_values(payload));
        values.extend(extract_ts4script_line_identity_values(payload));
    }

    unique_display_values(values)
}

fn extract_ts4script_json_identity_values(payload: &str) -> Vec<String> {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(payload) else {
        return Vec::new();
    };

    let mut values = Vec::new();
    for key in [
        "name",
        "mod_name",
        "mod-name",
        "modName",
        "title",
        "author",
        "authors",
        "creator",
        "creators",
        "author_name",
        "author-name",
        "authorName",
    ] {
        if let Some(candidate) = json.get(key) {
            values.extend(extract_ts4script_json_identity_field_values(candidate));
        }
    }

    values
}

fn extract_ts4script_json_identity_field_values(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::String(value) => {
            clean_payload_identity_value(value).into_iter().collect()
        }
        serde_json::Value::Array(values) => values
            .iter()
            .filter_map(|candidate| candidate.as_str())
            .filter_map(clean_payload_identity_value)
            .collect(),
        _ => Vec::new(),
    }
}

fn extract_ts4script_line_identity_values(payload: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut pending_list_key = false;

    for line in payload.lines().take(64) {
        let trimmed = line.trim();
        if pending_list_key {
            if let Some(value) = trimmed.strip_prefix('-') {
                if let Some(cleaned) = clean_payload_identity_value(value) {
                    values.push(cleaned);
                }
                continue;
            }
            if trimmed.is_empty() {
                continue;
            }
            pending_list_key = false;
        }

        let Some((key, value)) = line.split_once(':') else {
            continue;
        };

        let lowered_key = key.trim().to_ascii_lowercase();
        if !matches!(
            lowered_key.as_str(),
            "name"
                | "mod_name"
                | "mod-name"
                | "modname"
                | "title"
                | "author"
                | "authors"
                | "creator"
                | "creators"
                | "author_name"
                | "author-name"
                | "authorname"
        ) {
            pending_list_key = false;
            continue;
        }

        let trimmed_value = value.trim();
        if trimmed_value.is_empty() {
            pending_list_key = matches!(lowered_key.as_str(), "authors" | "creators");
            continue;
        }

        if trimmed_value.starts_with('[') && trimmed_value.ends_with(']') {
            for part in trimmed_value
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split(',')
            {
                if let Some(cleaned) = clean_payload_identity_value(part) {
                    values.push(cleaned);
                }
            }
            pending_list_key = false;
            continue;
        }

        if let Some(cleaned) = clean_payload_identity_value(trimmed_value) {
            values.push(cleaned);
        }
        pending_list_key = false;
    }

    values
}

fn clean_payload_identity_value(value: &str) -> Option<String> {
    let cleaned = value
        .trim()
        .trim_matches(|character| matches!(character, '"' | '\''))
        .trim()
        .trim_end_matches(',')
        .trim();

    if cleaned.is_empty()
        || cleaned.len() > 96
        || cleaned.eq_ignore_ascii_case("name")
        || cleaned.eq_ignore_ascii_case("version")
        || !cleaned
            .chars()
            .all(|character| character.is_ascii_graphic() || character.is_ascii_whitespace())
    {
        return None;
    }

    Some(cleaned.to_owned())
}

fn collect_family_hints<'a>(values: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut hints = BTreeSet::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }

        let lowered = trimmed.to_ascii_lowercase();
        if lowered.len() >= 3 {
            hints.insert(lowered.clone());
        }

        let spaced = lowered
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character
                } else {
                    ' '
                }
            })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if spaced.len() >= 3 {
            hints.insert(spaced.clone());
        }

        let compact = spaced.replace(' ', "");
        if compact.len() >= 4 {
            hints.insert(compact);
        }
    }
    hints.into_iter().take(MAX_DISPLAY_VALUES).collect()
}

fn parse_dbpf_header(file: &mut File) -> AppResult<DbpfHeader> {
    let mut header = [0_u8; DBPF_HEADER_SIZE];
    file.seek(SeekFrom::Start(0))?;
    file.read_exact(&mut header)?;

    if &header[0..4] != b"DBPF" {
        return Err(AppError::Message("Not a DBPF package".to_owned()));
    }

    let major = read_u32(&header[4..8])?;
    let minor = read_u32(&header[8..12])?;
    if major != 2 || minor != 1 {
        return Err(AppError::Message("Unsupported DBPF version".to_owned()));
    }

    Ok(DbpfHeader {
        record_count: read_u32(&header[36..40])?,
        index_size: read_u32(&header[44..48])?,
        index_offset: read_u32(&header[64..68])?,
    })
}

fn parse_dbpf_records(file: &mut File, header: DbpfHeader) -> AppResult<Vec<DbpfRecord>> {
    let mut buffer = vec![0_u8; header.index_size as usize];
    file.seek(SeekFrom::Start(header.index_offset as u64))?;
    file.read_exact(&mut buffer)?;

    let mut cursor = Cursor::new(buffer.as_slice());
    let common_mask = read_u32_from_cursor(&mut cursor)?;
    let mut common_values = [0_u32; 8];

    for (index, value) in common_values.iter_mut().enumerate() {
        if ((common_mask >> index) & 1) == 1 {
            *value = read_u32_from_cursor(&mut cursor)?;
        }
    }

    let mut records = Vec::with_capacity(header.record_count as usize);
    for _ in 0..header.record_count {
        let mut values = common_values;
        for (index, value) in values.iter_mut().enumerate() {
            if ((common_mask >> index) & 1) == 0 {
                *value = read_u32_from_cursor(&mut cursor)?;
            }
        }

        let compressed_reserved = values[7];
        records.push(DbpfRecord {
            resource_type: values[0],
            offset: values[4],
            packed_size: values[5],
            mem_size: values[6],
            compressed: ((compressed_reserved >> 16) & 0xffff) as u16,
        });
    }

    Ok(records)
}

fn collect_string_table_entries(
    file: &mut File,
    records: &[DbpfRecord],
) -> AppResult<BTreeMap<u32, String>> {
    let mut stbl_map = BTreeMap::new();

    for record in records
        .iter()
        .filter(|record| record.resource_type == RESOURCE_STRING_TABLE)
        .take(6)
    {
        let bytes = read_record_bytes(file, record)?;
        for (key, value) in parse_stbl_entries(&bytes)? {
            stbl_map.entry(key).or_insert(value);
        }
    }

    Ok(stbl_map)
}

fn collect_catalog_names(
    file: &mut File,
    records: &[DbpfRecord],
    stbl_map: &BTreeMap<u32, String>,
) -> AppResult<Vec<String>> {
    let mut names = Vec::new();

    for record in records
        .iter()
        .filter(|record| record.resource_type == RESOURCE_CATALOG)
        .take(8)
    {
        let bytes = read_record_bytes(file, record)?;
        if bytes.len() < 12 {
            continue;
        }

        let key = read_u32(&bytes[8..12])?;
        if let Some(value) = stbl_map.get(&key) {
            names.push(value.clone());
        }
    }

    Ok(unique_display_values(names))
}

fn collect_cas_part_names(file: &mut File, records: &[DbpfRecord]) -> AppResult<Vec<String>> {
    let mut names = Vec::new();

    for record in records
        .iter()
        .filter(|record| record.resource_type == RESOURCE_CAS_PART)
        .take(8)
    {
        let bytes = read_record_bytes(file, record)?;
        if bytes.len() <= 12 {
            continue;
        }

        if let Ok(name) = read_seven_bit_string_be(&bytes[12..]) {
            if !name.trim().is_empty() {
                names.push(name);
            }
        }
    }

    Ok(unique_display_values(names))
}

fn collect_name_map_values(file: &mut File, records: &[DbpfRecord]) -> AppResult<Vec<String>> {
    let mut values = Vec::new();

    for record in records
        .iter()
        .filter(|record| record.resource_type == RESOURCE_NAME_MAP)
        .take(4)
    {
        let bytes = read_record_bytes(file, record)?;
        values.extend(parse_name_map_entries(&bytes)?);
    }

    Ok(unique_display_values(values))
}

fn read_record_bytes(file: &mut File, record: &DbpfRecord) -> AppResult<Vec<u8>> {
    if record.packed_size == 0 || record.packed_size as usize > MAX_RESOURCE_BYTES {
        return Ok(Vec::new());
    }

    let mut bytes = vec![0_u8; record.packed_size as usize];
    file.seek(SeekFrom::Start(record.offset as u64))?;
    file.read_exact(&mut bytes)?;

    if !record.is_compressed() {
        return Ok(bytes);
    }

    decompress_record_bytes(record, &bytes)
}

fn decompress_record_bytes(record: &DbpfRecord, bytes: &[u8]) -> AppResult<Vec<u8>> {
    let expected_size = record
        .mem_size
        .max(record.packed_size)
        .min(MAX_RESOURCE_BYTES as u32) as usize;
    if expected_size == 0 || expected_size > MAX_RESOURCE_BYTES {
        return Ok(Vec::new());
    }
    if bytes.len() < 2 {
        return Ok(Vec::new());
    }

    match bytes[0] {
        0x78 => decompress_zlib(bytes, expected_size),
        _ if bytes[1] == 0xFB => decompress_legacy(bytes, expected_size),
        _ => Ok(Vec::new()),
    }
}

fn decompress_zlib(bytes: &[u8], expected_size: usize) -> AppResult<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(bytes);
    let mut output = Vec::with_capacity(expected_size);
    decoder.read_to_end(&mut output)?;
    if output.len() > MAX_RESOURCE_BYTES {
        return Ok(Vec::new());
    }
    Ok(output)
}

fn decompress_legacy(bytes: &[u8], expected_size: usize) -> AppResult<Vec<u8>> {
    let compression_type = bytes[0];
    let mut cursor = Cursor::new(&bytes[2..]);
    let encoded_size = read_legacy_size(&mut cursor, compression_type)?;
    let target_size = encoded_size.max(expected_size).min(MAX_RESOURCE_BYTES);
    let mut output = vec![0_u8; target_size];
    let mut position = 0_usize;

    while position < output.len() {
        let byte0 = read_u8_from_cursor(&mut cursor)?;
        match byte0 {
            0x00..=0x7f => {
                let byte1 = read_u8_from_cursor(&mut cursor)?;
                let num_plain_text = (byte0 & 0x03) as usize;
                let num_to_copy = (((byte0 & 0x1c) >> 2) + 3) as usize;
                let copy_offset = (((byte0 & 0x60) as usize) << 3) + byte1 as usize + 1;

                copy_plain_text(&mut cursor, &mut output, num_plain_text, &mut position)?;
                copy_compressed_text(&mut output, num_to_copy, &mut position, copy_offset)?;
            }
            0x80..=0xbf => {
                let byte1 = read_u8_from_cursor(&mut cursor)?;
                let byte2 = read_u8_from_cursor(&mut cursor)?;
                let num_plain_text = ((byte1 >> 6) & 0x03) as usize;
                let num_to_copy = ((byte0 & 0x3f) + 4) as usize;
                let copy_offset = (((byte1 & 0x3f) as usize) << 8) + byte2 as usize + 1;

                copy_plain_text(&mut cursor, &mut output, num_plain_text, &mut position)?;
                copy_compressed_text(&mut output, num_to_copy, &mut position, copy_offset)?;
            }
            0xc0..=0xdf => {
                let byte1 = read_u8_from_cursor(&mut cursor)?;
                let byte2 = read_u8_from_cursor(&mut cursor)?;
                let byte3 = read_u8_from_cursor(&mut cursor)?;
                let num_plain_text = (byte0 & 0x03) as usize;
                let num_to_copy = ((((byte0 & 0x0c) as usize) << 6) + byte3 as usize + 5) as usize;
                let copy_offset = (((byte0 & 0x10) as usize) << 12)
                    + ((byte1 as usize) << 8)
                    + byte2 as usize
                    + 1;

                copy_plain_text(&mut cursor, &mut output, num_plain_text, &mut position)?;
                copy_compressed_text(&mut output, num_to_copy, &mut position, copy_offset)?;
            }
            0xe0..=0xfb => {
                let num_plain_text = (((byte0 & 0x1f) as usize) << 2) + 4;
                copy_plain_text(&mut cursor, &mut output, num_plain_text, &mut position)?;
            }
            0xfc..=0xff => {
                let num_plain_text = (byte0 & 0x03) as usize;
                copy_plain_text(&mut cursor, &mut output, num_plain_text, &mut position)?;
            }
        }
    }

    Ok(output)
}

fn read_legacy_size(cursor: &mut Cursor<&[u8]>, compression_type: u8) -> AppResult<usize> {
    let three_byte_length = compression_type != 0x80;
    let mut size_bytes = [0_u8; 4];

    let start = if three_byte_length { 2 } else { 3 };
    for index in (0..=start).rev() {
        size_bytes[index] = read_u8_from_cursor(cursor)?;
    }

    Ok(u32::from_le_bytes(size_bytes) as usize)
}

fn copy_plain_text(
    cursor: &mut Cursor<&[u8]>,
    output: &mut [u8],
    count: usize,
    position: &mut usize,
) -> AppResult<()> {
    for _ in 0..count {
        if *position >= output.len() {
            return Err(AppError::Message(
                "Compressed payload overran target buffer".to_owned(),
            ));
        }

        output[*position] = read_u8_from_cursor(cursor)?;
        *position += 1;
    }

    Ok(())
}

fn copy_compressed_text(
    output: &mut [u8],
    count: usize,
    position: &mut usize,
    copy_offset: usize,
) -> AppResult<()> {
    if copy_offset == 0 || copy_offset > *position {
        return Err(AppError::Message(
            "Compressed payload referenced invalid back-copy offset".to_owned(),
        ));
    }

    let current_position = *position;
    for index in 0..count {
        if *position >= output.len() {
            return Err(AppError::Message(
                "Compressed payload overran target buffer".to_owned(),
            ));
        }

        output[*position] = output[current_position - copy_offset + index];
        *position += 1;
    }

    Ok(())
}

/// Builds a concise, simmer-facing resource summary for a DBPF package.
///
/// Important honesty rule: DBPF record counts are not the same thing as logical
/// in-game items. For Build/Buy content we use the strongest safe proxy we have
/// from existing extraction data, and keep the wording broad enough to avoid
/// overstating certainty.
fn build_resource_summary(
    type_counts: &BTreeMap<u32, usize>,
    _records: &[DbpfRecord],
) -> Vec<String> {
    let mut summary = Vec::new();

    let catalog_count = *type_counts.get(&RESOURCE_CATALOG).unwrap_or(&0);
    let definition_count = *type_counts.get(&RESOURCE_DEFINITION).unwrap_or(&0);
    let string_table_count = *type_counts.get(&RESOURCE_STRING_TABLE).unwrap_or(&0);
    let build_buy_proxy = [catalog_count, definition_count, string_table_count]
        .into_iter()
        .max()
        .unwrap_or(0);
    if build_buy_proxy > 0 {
        summary.push(format!(
            "{} build/buy item{}",
            build_buy_proxy,
            if build_buy_proxy == 1 { "" } else { "s" }
        ));
    }

    let cas_part_count = *type_counts.get(&RESOURCE_CAS_PART).unwrap_or(&0);
    let skintone_count = *type_counts.get(&RESOURCE_SKINTONE).unwrap_or(&0);
    let cas_total = cas_part_count + skintone_count;
    if cas_total > 0 {
        summary.push(format!(
            "{} CAS part{}",
            cas_total,
            if cas_total == 1 { "" } else { "s" }
        ));
    }

    if let Some(&count) = type_counts.get(&RESOURCE_SCRIPT) {
        summary.push(format!(
            "{} script resource{}",
            count,
            if count == 1 { "" } else { "s" }
        ));
    }

    let known_types = [
        RESOURCE_CATALOG,
        RESOURCE_DEFINITION,
        RESOURCE_STRING_TABLE,
        RESOURCE_CAS_PART,
        RESOURCE_SKINTONE,
        RESOURCE_SCRIPT,
        RESOURCE_HOTSPOT,
        RESOURCE_NAME_MAP,
    ];
    let unknown_total: usize = type_counts
        .iter()
        .filter(|(rt, _)| !known_types.contains(rt))
        .map(|(_, c)| c)
        .sum();
    if unknown_total > 0 {
        summary.push(format!(
            "{} other resource{}",
            unknown_total,
            if unknown_total == 1 { "" } else { "s" }
        ));
    }

    summary.truncate(MAX_DISPLAY_VALUES);
    summary
}

/// Builds a human-readable content profile for ts4script files,
/// replacing generic "Archive entries: N" strings with meaningful content signals.
fn build_ts4script_resource_summary(
    archive_paths: &[String],
    namespace_count: usize,
) -> Vec<String> {
    let mut summary = Vec::new();
    summary.push("Script mod".to_owned());

    // Detect content types from file extensions in archive paths
    let has_python = archive_paths.iter().any(|p| p.to_ascii_lowercase().ends_with(".py"));
    let has_yaml = archive_paths.iter().any(|p| p.to_ascii_lowercase().ends_with(".yml") || p.to_ascii_lowercase().ends_with(".yaml"));
    let has_mod_manifest = archive_paths.iter().any(|p| {
        let lower = p.to_ascii_lowercase();
        lower.contains("modfilemanifest")
    });
    let has_json = archive_paths.iter().any(|p| p.to_ascii_lowercase().ends_with(".json"));

    if has_python {
        summary.push("Python modules".to_owned());
    }
    // Only show "Config files" when there are actual YAML files, not just the manifest
    // (modfilemanifest is present in all ts4script archives and is not a user config file)
    if has_yaml {
        summary.push("YAML config".to_owned());
    }
    // Mod manifest: indicates this is a properly structured script mod with a descriptor file
    if has_mod_manifest && !has_yaml {
        summary.push("Script config".to_owned());
    }
    if has_json && !has_yaml {
        summary.push("JSON data".to_owned());
    }

    if namespace_count > 1 {
        summary.push(format!("{} namespaces", namespace_count));
    }

    summary
}

#[derive(Debug, Clone, Default)]
struct ResourceKindHint {
    kind_hint: Option<String>,
    subtype_hint: Option<String>,
    confidence_floor: f64,
}

fn infer_kind_from_package_signals(
    path: &Path,
    type_counts: &BTreeMap<u32, usize>,
) -> ResourceKindHint {
    let mut resource_kind = infer_kind_from_resources(type_counts);
    if resource_kind.kind_hint.is_none() {
        let support_hint = infer_string_support_kind(path, type_counts);
        if support_hint.kind_hint.is_some() {
            resource_kind = support_hint;
        }
    }
    if resource_kind.kind_hint.is_none() {
        let gameplay_hint = infer_gameplay_context_kind(path, type_counts);
        if gameplay_hint.kind_hint.is_some() {
            resource_kind = gameplay_hint;
        }
    }
    resource_kind
}

fn infer_kind_from_resources(type_counts: &BTreeMap<u32, usize>) -> ResourceKindHint {
    let cas_score = type_counts
        .get(&RESOURCE_CAS_PART)
        .copied()
        .unwrap_or_default()
        + type_counts
            .get(&RESOURCE_SKINTONE)
            .copied()
            .unwrap_or_default()
        + lean_cas_appearance_score(type_counts);
    let has_build_surfaces =
        count_present_resource_types(type_counts, BUILD_SURFACE_RESOURCE_TYPES) > 0;
    let has_build_structures =
        count_present_resource_types(type_counts, BUILD_STRUCTURE_RESOURCE_TYPES) > 0;
    let build_buy_score = type_counts
        .get(&RESOURCE_CATALOG)
        .copied()
        .unwrap_or_default()
        + type_counts
            .get(&RESOURCE_DEFINITION)
            .copied()
            .unwrap_or_default()
        + if has_build_surfaces { 4 } else { 0 }
        + if has_build_structures { 4 } else { 0 };
    let preset_score = type_counts
        .get(&RESOURCE_HOTSPOT)
        .copied()
        .unwrap_or_default();
    let script_score = type_counts
        .get(&RESOURCE_SCRIPT)
        .copied()
        .unwrap_or_default();
    let gameplay_score = gameplay_resource_score(type_counts);

    let mut candidates = vec![
        (
            "CAS",
            cas_score,
            if type_counts.contains_key(&RESOURCE_SKINTONE) {
                Some("Skin")
            } else {
                None
            },
            0.72,
        ),
        (
            "BuildBuy",
            build_buy_score,
            if has_build_surfaces || has_build_structures {
                Some("Build Surfaces")
            } else {
                None
            },
            if has_build_surfaces || has_build_structures {
                0.7
            } else {
                0.68
            },
        ),
        ("PresetsAndSliders", preset_score, Some("Sliders"), 0.68),
        ("ScriptMods", script_score, Some("Utilities"), 0.7),
    ];
    if gameplay_score >= 3 {
        candidates.push((
            "Gameplay",
            gameplay_score,
            Some("Gameplay"),
            if gameplay_score >= 8 { 0.64 } else { 0.58 },
        ));
    }
    candidates.sort_by(|left, right| right.1.cmp(&left.1));

    match candidates.first() {
        Some((kind, score, subtype, confidence_floor)) if *score > 0 => ResourceKindHint {
            kind_hint: Some((*kind).to_owned()),
            subtype_hint: subtype.map(|value| value.to_owned()),
            confidence_floor: *confidence_floor,
        },
        _ => ResourceKindHint::default(),
    }
}

fn count_present_resource_types(
    type_counts: &BTreeMap<u32, usize>,
    resource_types: &[u32],
) -> usize {
    resource_types
        .iter()
        .filter(|resource_type| type_counts.contains_key(resource_type))
        .count()
}

fn lean_cas_appearance_score(type_counts: &BTreeMap<u32, usize>) -> usize {
    if type_counts.len() > 4
        || type_counts.contains_key(&RESOURCE_CATALOG)
        || type_counts.contains_key(&RESOURCE_DEFINITION)
        || type_counts.contains_key(&RESOURCE_STRING_TABLE)
        || type_counts.contains_key(&RESOURCE_SCRIPT)
    {
        return 0;
    }

    let appearance_signals =
        count_present_resource_types(type_counts, LEAN_CAS_APPEARANCE_RESOURCE_TYPES);
    if appearance_signals == 0 {
        return 0;
    }

    if type_counts
        .keys()
        .any(|resource_type| BUILD_SURFACE_RESOURCE_TYPES.contains(resource_type))
        || type_counts
            .keys()
            .any(|resource_type| BUILD_STRUCTURE_RESOURCE_TYPES.contains(resource_type))
    {
        return 0;
    }

    4
}

fn gameplay_resource_score(type_counts: &BTreeMap<u32, usize>) -> usize {
    GAMEPLAY_RESOURCE_WEIGHTS
        .iter()
        .filter_map(|(resource_type, weight)| {
            let count = type_counts.get(resource_type).copied().unwrap_or_default();
            (count > 0).then_some(
                *weight
                    + match count {
                        0 | 1 => 0,
                        2..=3 => 1,
                        4..=11 => 2,
                        _ => 3,
                    },
            )
        })
        .sum()
}

fn infer_string_support_kind(path: &Path, type_counts: &BTreeMap<u32, usize>) -> ResourceKindHint {
    if !is_string_support_focused_package(type_counts) {
        return ResourceKindHint::default();
    }

    let context_tokens = package_context_tokens(path);
    if context_tokens.iter().any(|token| {
        STRING_SUPPORT_HINT_TOKENS
            .iter()
            .any(|candidate| candidate == &token.as_str())
    }) {
        return ResourceKindHint {
            kind_hint: Some("Gameplay".to_owned()),
            subtype_hint: Some("Utilities".to_owned()),
            confidence_floor: 0.58,
        };
    }

    ResourceKindHint::default()
}

fn is_string_table_only_package(type_counts: &BTreeMap<u32, usize>) -> bool {
    type_counts.len() == 1 && type_counts.contains_key(&RESOURCE_STRING_TABLE)
}

fn is_string_support_focused_package(type_counts: &BTreeMap<u32, usize>) -> bool {
    if is_string_table_only_package(type_counts) {
        return true;
    }

    type_counts.contains_key(&RESOURCE_STRING_TABLE)
        && !type_counts.contains_key(&RESOURCE_CAS_PART)
        && !type_counts.contains_key(&RESOURCE_SKINTONE)
        && !type_counts.contains_key(&RESOURCE_CATALOG)
        && !type_counts.contains_key(&RESOURCE_DEFINITION)
        && !type_counts.contains_key(&RESOURCE_HOTSPOT)
        && !type_counts.contains_key(&RESOURCE_SCRIPT)
}

fn infer_gameplay_context_kind(
    path: &Path,
    type_counts: &BTreeMap<u32, usize>,
) -> ResourceKindHint {
    let context_tokens = package_context_tokens(path);
    let has_gameplay_context_token = context_tokens.iter().any(|token| {
        GAMEPLAY_CONTEXT_HINT_TOKENS
            .iter()
            .any(|candidate| candidate == &token.as_str())
    });
    let has_dense_lot_price_cluster = context_tokens.iter().any(|token| {
        token == "lot" || token == "price" || token == "prices" || token == "lotprices"
    }) && type_counts.len() == 1
        && type_counts.values().next().copied().unwrap_or_default() >= 64;
    if !has_gameplay_context_token && !has_dense_lot_price_cluster {
        return ResourceKindHint::default();
    }

    let gameplay_score =
        gameplay_resource_score(type_counts) + usize::from(type_counts.contains_key(&0x62ec_c59a));

    if gameplay_score >= 1 || has_dense_lot_price_cluster {
        return ResourceKindHint {
            kind_hint: Some("Gameplay".to_owned()),
            subtype_hint: Some("Gameplay".to_owned()),
            confidence_floor: if gameplay_score >= 2 || has_dense_lot_price_cluster {
                0.58
            } else {
                0.56
            },
        };
    }

    ResourceKindHint::default()
}

fn package_context_tokens(path: &Path) -> Vec<String> {
    let mut tokens = Vec::new();

    for component in path
        .components()
        .rev()
        .take(4)
        .filter_map(|component| component.as_os_str().to_str())
    {
        let component_tokens = tokenize_context_component(component);
        for token in &component_tokens {
            if !token.is_empty() && !tokens.contains(token) {
                tokens.push(token.clone());
            }
        }
        for compact in compact_context_windows(&component_tokens) {
            if !tokens.contains(&compact) {
                tokens.push(compact);
            }
        }
    }

    tokens
}

fn compact_context_windows(tokens: &[String]) -> Vec<String> {
    let mut compact = Vec::new();
    for start in 0..tokens.len() {
        for width in 2..=3 {
            if start + width > tokens.len() {
                break;
            }

            let candidate = tokens[start..(start + width)].join("");
            if candidate.len() >= 6 {
                compact.push(candidate);
            }
        }
    }

    compact
}

fn tokenize_context_component(value: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let characters = value.chars().collect::<Vec<_>>();

    for index in 0..characters.len() {
        let current_char = characters[index];
        if !current_char.is_ascii_alphanumeric() {
            flush_context_token(&mut current, &mut tokens);
            continue;
        }

        if should_split_context_token_before(&characters, index) {
            flush_context_token(&mut current, &mut tokens);
        }

        current.push(current_char.to_ascii_lowercase());
    }

    flush_context_token(&mut current, &mut tokens);
    tokens
}

fn should_split_context_token_before(characters: &[char], index: usize) -> bool {
    if index == 0 {
        return false;
    }

    let current = characters[index];
    let previous = characters[index - 1];
    current.is_ascii_uppercase()
        && (previous.is_ascii_lowercase()
            || (previous.is_ascii_uppercase()
                && characters
                    .get(index + 1)
                    .copied()
                    .is_some_and(|next| next.is_ascii_lowercase())))
}

fn flush_context_token(current: &mut String, tokens: &mut Vec<String>) {
    if !current.is_empty() {
        tokens.push(std::mem::take(current));
    }
}

fn collect_creator_hints<'a, I>(values: I, seed_pack: &SeedPack) -> Vec<String>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut known_hints = Vec::new();
    let mut fallback_hints = Vec::new();
    for value in values {
        if let Some(creator) = detect_creator_hint(value, seed_pack) {
            let already_recorded = known_hints.iter().any(|existing| existing == &creator)
                || fallback_hints.iter().any(|existing| existing == &creator);
            if already_recorded {
                continue;
            }

            let is_known = seed_pack.creator_profiles.contains_key(&creator)
                || seed_pack
                    .creator_lookup
                    .values()
                    .any(|existing| existing == &creator);
            if is_known {
                known_hints.push(creator);
            } else if creator.chars().count() >= MIN_FALLBACK_CREATOR_HINT_LEN {
                fallback_hints.push(creator);
            } else {
                continue;
            }
        }

        if known_hints.len() + fallback_hints.len() >= MAX_CREATOR_HINTS {
            break;
        }
    }

    known_hints
        .into_iter()
        .chain(fallback_hints)
        .take(MAX_CREATOR_HINTS)
        .collect()
}

fn unique_display_values(values: Vec<String>) -> Vec<String> {
    let mut unique = Vec::new();
    for value in values {
        let cleaned = value.trim();
        if cleaned.is_empty()
            || cleaned.len() > 96
            || !cleaned
                .chars()
                .all(|character| character.is_ascii_graphic() || character.is_ascii_whitespace())
        {
            continue;
        }

        if !unique
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(cleaned))
        {
            unique.push(cleaned.to_owned());
        }

        if unique.len() >= MAX_DISPLAY_VALUES {
            break;
        }
    }

    unique
}

fn resource_label(resource_type: u32) -> Option<&'static str> {
    match resource_type {
        RESOURCE_NAME_MAP => Some("NameMap"),
        RESOURCE_STRING_TABLE => Some("StringTable"),
        RESOURCE_CAS_PART => Some("CASPart"),
        RESOURCE_SKINTONE => Some("Skintone"),
        RESOURCE_CATALOG => Some("Catalog"),
        RESOURCE_DEFINITION => Some("Definition"),
        RESOURCE_HOTSPOT => Some("HotSpotControl"),
        RESOURCE_SCRIPT => Some("ScriptResource"),
        _ => None,
    }
}

fn parse_name_map_entries(bytes: &[u8]) -> AppResult<Vec<String>> {
    if bytes.len() < 8 {
        return Ok(Vec::new());
    }

    let mut cursor = Cursor::new(bytes);
    let _version = read_u32_from_cursor(&mut cursor)?;
    let count = read_u32_from_cursor(&mut cursor)?;

    let mut values = Vec::new();
    for _ in 0..count {
        if cursor.position() + 12 > bytes.len() as u64 {
            break;
        }

        let _key = read_u64_from_cursor(&mut cursor)?;
        let length = read_u32_from_cursor(&mut cursor)? as usize;
        if length > MAX_RESOURCE_BYTES {
            break;
        }

        if let Some(value) = read_utf8_chars(&mut cursor, bytes, length) {
            values.push(value);
        } else {
            break;
        }
    }

    Ok(values)
}

fn parse_stbl_entries(bytes: &[u8]) -> AppResult<Vec<(u32, String)>> {
    if bytes.len() < 21 {
        return Ok(Vec::new());
    }

    let mut cursor = Cursor::new(bytes);
    let magic = read_u32_from_cursor(&mut cursor)?;
    if magic != u32::from_le_bytes(*b"STBL") {
        return Ok(Vec::new());
    }

    let _version = read_u8_from_cursor(&mut cursor)?;
    let _unknown = read_u16_from_cursor(&mut cursor)?;
    let count = read_u32_from_cursor(&mut cursor)?;

    let mut unknown2 = [0_u8; 6];
    cursor.read_exact(&mut unknown2)?;
    let _size = read_u32_from_cursor(&mut cursor)?;

    let mut entries = Vec::new();
    for _ in 0..count {
        if cursor.position() + 7 > bytes.len() as u64 {
            break;
        }

        let key = read_u32_from_cursor(&mut cursor)?;
        let _flags = read_u8_from_cursor(&mut cursor)?;
        let length = read_u16_from_cursor(&mut cursor)? as usize;
        if cursor.position() + length as u64 > bytes.len() as u64 {
            break;
        }

        let mut raw = vec![0_u8; length];
        cursor.read_exact(&mut raw)?;
        entries.push((key, String::from_utf8_lossy(&raw).to_string()));
    }

    Ok(entries)
}

fn read_seven_bit_string_be(bytes: &[u8]) -> AppResult<String> {
    let mut cursor = Cursor::new(bytes);
    let mut length = 0_usize;
    let mut shift = 0_usize;

    loop {
        let byte = read_u8_from_cursor(&mut cursor)? as usize;
        length |= (byte & 0x7f) << shift;
        if (byte & 0x80) == 0 {
            break;
        }
        shift += 7;
        if shift > 28 {
            return Err(AppError::Message(
                "Invalid seven-bit encoded string".to_owned(),
            ));
        }
    }

    if cursor.position() + length as u64 > bytes.len() as u64 {
        return Err(AppError::Message(
            "String length exceeded buffer".to_owned(),
        ));
    }

    let mut raw = vec![0_u8; length];
    cursor.read_exact(&mut raw)?;
    Ok(decode_utf16_be(&raw))
}

fn decode_utf16_be(bytes: &[u8]) -> String {
    String::from_utf16_lossy(
        &bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>(),
    )
}

fn read_utf8_chars(cursor: &mut Cursor<&[u8]>, source: &[u8], char_count: usize) -> Option<String> {
    let mut raw = Vec::new();

    while cursor.position() < source.len() as u64 {
        raw.push(read_u8_from_cursor(cursor).ok()?);
        let text = std::str::from_utf8(&raw).ok()?;
        if text.chars().count() == char_count {
            return Some(text.to_owned());
        }
    }

    None
}

fn read_u8_from_cursor(cursor: &mut Cursor<&[u8]>) -> AppResult<u8> {
    let mut bytes = [0_u8; 1];
    cursor.read_exact(&mut bytes)?;
    Ok(bytes[0])
}

fn read_u16_from_cursor(cursor: &mut Cursor<&[u8]>) -> AppResult<u16> {
    let mut bytes = [0_u8; 2];
    cursor.read_exact(&mut bytes)?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32_from_cursor(cursor: &mut Cursor<&[u8]>) -> AppResult<u32> {
    let mut bytes = [0_u8; 4];
    cursor.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64_from_cursor(cursor: &mut Cursor<&[u8]>) -> AppResult<u64> {
    let mut bytes = [0_u8; 8];
    cursor.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

fn read_u32(bytes: &[u8]) -> AppResult<u32> {
    let array: [u8; 4] = bytes
        .try_into()
        .map_err(|_| AppError::Message("Expected four bytes".to_owned()))?;
    Ok(u32::from_le_bytes(array))
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fs, fs::File, io::Write, path::Path};

    use flate2::{write::ZlibEncoder, Compression};
    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;

    use crate::seed::load_seed_pack;

    use super::{
        build_resource_summary, decompress_legacy, decompress_record_bytes,
        infer_kind_from_package_signals, infer_kind_from_resources, inspect_file,
        parse_name_map_entries, parse_stbl_entries, read_seven_bit_string_be, DbpfRecord,
        RESOURCE_CAS_PART, RESOURCE_CATALOG, RESOURCE_DEFINITION, RESOURCE_SKINTONE,
        RESOURCE_STRING_TABLE,
    };

    #[test]
    fn parses_name_map_entries() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u32.to_le_bytes());
        bytes.extend_from_slice(&42_u64.to_le_bytes());
        bytes.extend_from_slice(&6_u32.to_le_bytes());
        bytes.extend_from_slice(b"Miiko!");

        let values = parse_name_map_entries(&bytes).expect("name map");
        assert_eq!(values, vec!["Miiko!".to_owned()]);
    }

    #[test]
    fn parses_stbl_entries() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"STBL");
        bytes.push(0x05);
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u32.to_le_bytes());
        bytes.extend_from_slice(&[0_u8; 6]);
        bytes.extend_from_slice(&8_u32.to_le_bytes());
        bytes.extend_from_slice(&0x1234_5678_u32.to_le_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&7_u16.to_le_bytes());
        bytes.extend_from_slice(b"Counter");

        let entries = parse_stbl_entries(&bytes).expect("stbl");
        assert_eq!(entries[0].0, 0x1234_5678);
        assert_eq!(entries[0].1, "Counter");
    }

    #[test]
    fn reads_big_endian_prefixed_strings() {
        let bytes = [0x08, 0x00, 0x4d, 0x00, 0x69, 0x00, 0x69, 0x00, 0x6b];
        let value = read_seven_bit_string_be(&bytes).expect("value");
        assert_eq!(value, "Miik");
    }

    #[test]
    fn inspects_ts4script_archives_for_namespaces_and_creators() {
        let seed_pack = load_seed_pack().expect("seed");
        let temp = tempdir().expect("tempdir");
        let filepath = temp.path().join("BetterExceptions.ts4script");
        let file = File::create(&filepath).expect("archive");
        let mut writer = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        writer
            .start_file("twistedmexi/better_exceptions/__init__.pyc", options)
            .expect("start");
        writer.write_all(b"pyc").expect("write");
        writer.finish().expect("finish");

        let outcome = inspect_file(&filepath, ".ts4script", &seed_pack).expect("inspect");
        assert_eq!(outcome.creator_hint.as_deref(), Some("TwistedMexi"));
        assert_eq!(outcome.kind_hint.as_deref(), Some("ScriptMods"));
        assert!(outcome
            .insights
            .script_namespaces
            .iter()
            .any(|value| value == "twistedmexi"));

        fs::remove_file(filepath).expect("cleanup");
    }

    #[test]
    fn inspects_ts4script_archives_for_version_and_family_hints() {
        let seed_pack = load_seed_pack().expect("seed");
        let temp = tempdir().expect("tempdir");
        let filepath = temp
            .path()
            .join("McCmdCenter_AllModules_2026_1_1.ts4script");
        let file = File::create(&filepath).expect("archive");
        let mut writer = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        writer
            .start_file("deaderpool/mccc/mc_cmd_center.pyc", options)
            .expect("start");
        writer.write_all(b"pyc").expect("write");
        writer.finish().expect("finish");

        let outcome = inspect_file(&filepath, ".ts4script", &seed_pack).expect("inspect");
        assert!(outcome
            .insights
            .version_hints
            .iter()
            .any(|value| value == "2026.1.1"));
        assert!(outcome
            .insights
            .family_hints
            .iter()
            .any(|value| value == "mccc"));

        fs::remove_file(filepath).expect("cleanup");
    }

    #[test]
    fn inspects_ts4script_payloads_for_internal_version_hints() {
        let seed_pack = load_seed_pack().expect("seed");
        let temp = tempdir().expect("tempdir");
        let filepath = temp.path().join("McCmdCenter_AllModules.ts4script");
        let file = File::create(&filepath).expect("archive");
        let mut writer = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        writer
            .start_file("deaderpool/mccc/mc_cmd_version.pyc", options)
            .expect("start");
        writer
            .write_all(b"\0release 1.113.277 and current version 2026_1_1")
            .expect("write");
        writer.finish().expect("finish");

        let outcome = inspect_file(&filepath, ".ts4script", &seed_pack).expect("inspect");
        assert!(outcome
            .insights
            .version_hints
            .iter()
            .any(|value| value == "2026.1.1"));

        fs::remove_file(filepath).expect("cleanup");
    }

    #[test]
    fn ts4script_manifest_versions_beat_game_patch_noise() {
        let seed_pack = load_seed_pack().expect("seed");
        let temp = tempdir().expect("tempdir");
        let filepath = temp.path().join("lot51_core.ts4script");
        let file = File::create(&filepath).expect("archive");
        let mut writer = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        writer
            .start_file("lot51_core/lib/game_version.pyc", options)
            .expect("start game version");
        writer
            .write_all(b"Supports game version 1.105.332")
            .expect("write game version");
        writer
            .start_file("lot51_core/llamalogic.modfilemanifest.yml", options)
            .expect("start manifest");
        writer
            .write_all(b"name: Lot 51 Core Library\nversion: 1.41\n")
            .expect("write manifest");
        writer.finish().expect("finish");

        let outcome = inspect_file(&filepath, ".ts4script", &seed_pack).expect("inspect");
        assert!(outcome
            .insights
            .version_hints
            .iter()
            .any(|value| value == "1.41"));
        assert!(!outcome
            .insights
            .version_hints
            .iter()
            .any(|value| value == "1.105.332"));

        fs::remove_file(filepath).expect("cleanup");
    }

    #[test]
    fn ts4script_manifest_names_feed_identity_hints() {
        let seed_pack = load_seed_pack().expect("seed");
        let temp = tempdir().expect("tempdir");
        let filepath = temp.path().join("manifest_named_mod.ts4script");
        let file = File::create(&filepath).expect("archive");
        let mut writer = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        writer
            .start_file("manifest.json", options)
            .expect("start manifest");
        writer
            .write_all(br#"{ "name": "Better Exceptions" }"#)
            .expect("write manifest");
        writer.finish().expect("finish");

        let outcome = inspect_file(&filepath, ".ts4script", &seed_pack).expect("inspect");
        assert!(outcome
            .insights
            .embedded_names
            .iter()
            .any(|value| value == "Better Exceptions"));
        assert!(outcome
            .insights
            .family_hints
            .iter()
            .any(|value| value == "better exceptions"));

        fs::remove_file(filepath).expect("cleanup");
    }

    #[test]
    fn ts4script_manifest_authors_feed_creator_hints() {
        let seed_pack = load_seed_pack().expect("seed");
        let temp = tempdir().expect("tempdir");
        let filepath = temp.path().join("manifest_author_mod.ts4script");
        let file = File::create(&filepath).expect("archive");
        let mut writer = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        writer
            .start_file("manifest.json", options)
            .expect("start manifest");
        writer
            .write_all(
                br#"{ "name": "Mystery Utility", "authors": ["TwistedMexi", "Helper Friend"] }"#,
            )
            .expect("write manifest");
        writer.finish().expect("finish");

        let outcome = inspect_file(&filepath, ".ts4script", &seed_pack).expect("inspect");
        assert_eq!(outcome.creator_hint.as_deref(), Some("TwistedMexi"));
        assert!(outcome
            .insights
            .creator_hints
            .iter()
            .any(|value| value == "TwistedMexi"));

        fs::remove_file(filepath).expect("cleanup");
    }

    #[test]
    fn ts4script_manifest_author_lists_feed_creator_hints() {
        let seed_pack = load_seed_pack().expect("seed");
        let temp = tempdir().expect("tempdir");
        let filepath = temp.path().join("manifest_author_list_mod.ts4script");
        let file = File::create(&filepath).expect("archive");
        let mut writer = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        writer
            .start_file("manifest.yml", options)
            .expect("start manifest");
        writer
            .write_all(b"name: Mystery Utility\nauthors:\n  - TwistedMexi\n  - Helper Friend\n")
            .expect("write manifest");
        writer.finish().expect("finish");

        let outcome = inspect_file(&filepath, ".ts4script", &seed_pack).expect("inspect");
        assert_eq!(outcome.creator_hint.as_deref(), Some("TwistedMexi"));
        assert!(outcome
            .insights
            .creator_hints
            .iter()
            .any(|value| value == "TwistedMexi"));

        fs::remove_file(filepath).expect("cleanup");
    }

    #[test]
    fn flat_ts4script_archives_skip_filename_noise_in_script_clues() {
        let seed_pack = load_seed_pack().expect("seed");
        let temp = tempdir().expect("tempdir");
        let filepath = temp.path().join("mc_career.ts4script");
        let file = File::create(&filepath).expect("archive");
        let mut writer = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        writer
            .start_file("_DO_NOT_UNZIP_.txt", options)
            .expect("start marker");
        writer.write_all(b"Do not unzip").expect("write marker");
        writer
            .start_file("mc_career.pyc", options)
            .expect("start main");
        writer.write_all(b"pyc").expect("write main");
        writer
            .start_file("mc_career_version.pyc", options)
            .expect("start version");
        writer.write_all(b"pyc").expect("write version");
        writer.finish().expect("finish");

        let outcome = inspect_file(&filepath, ".ts4script", &seed_pack).expect("inspect");
        assert!(outcome
            .insights
            .script_namespaces
            .iter()
            .all(|value| !value.ends_with(".pyc") && !value.ends_with(".txt")));
        assert!(!outcome
            .insights
            .embedded_names
            .iter()
            .any(|value| value == "_DO_NOT_UNZIP_"));
        assert!(!outcome
            .insights
            .creator_hints
            .iter()
            .any(|value| value.eq_ignore_ascii_case("mc")));

        fs::remove_file(filepath).expect("cleanup");
    }

    #[test]
    fn decompresses_zlib_record_payloads() {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"Counter").expect("write");
        let compressed = encoder.finish().expect("finish");
        let record = DbpfRecord {
            resource_type: RESOURCE_STRING_TABLE,
            offset: 0,
            packed_size: compressed.len() as u32,
            mem_size: 7,
            compressed: 0x5A42,
        };

        let payload = decompress_record_bytes(&record, &compressed).expect("payload");
        assert_eq!(payload, b"Counter");
    }

    #[test]
    fn decompresses_legacy_record_payloads() {
        let compressed = vec![
            0x10, 0xFB, 0x00, 0x00, 0x05, 0xE0, b'H', b'e', b'l', b'l', 0xFD, b'o',
        ];
        let payload = decompress_legacy(&compressed, 5).expect("payload");
        assert_eq!(payload, b"Hello");
    }

    #[test]
    fn resource_summary_uses_safer_build_buy_proxy_labels() {
        let mut type_counts = BTreeMap::new();
        type_counts.insert(RESOURCE_CATALOG, 6);
        type_counts.insert(RESOURCE_DEFINITION, 6);
        type_counts.insert(RESOURCE_STRING_TABLE, 1);
        type_counts.insert(RESOURCE_CAS_PART, 2);
        type_counts.insert(RESOURCE_SKINTONE, 1);

        let summary = build_resource_summary(&type_counts, &[]);

        assert_eq!(summary.first().map(String::as_str), Some("6 build/buy items"));
        assert!(summary.iter().any(|item| item == "3 CAS parts"));
    }

    #[test]
    fn infers_build_surfaces_from_structural_resource_clusters() {
        let type_counts = BTreeMap::from([
            (0x0201_9972, 1_usize),
            (0x2FAE_983E, 12_usize),
            (0x76BC_F80C, 1_usize),
        ]);

        let hint = infer_kind_from_resources(&type_counts);
        assert_eq!(hint.kind_hint.as_deref(), Some("BuildBuy"));
        assert_eq!(hint.subtype_hint.as_deref(), Some("Build Surfaces"));
        assert!(hint.confidence_floor >= 0.68);
    }

    #[test]
    fn infers_gameplay_from_repeated_tuning_resource_clusters() {
        let type_counts = BTreeMap::from([
            (0x0C77_2E27, 24_usize),
            (0x545A_C67A, 18_usize),
            (0x6017_E896, 6_usize),
            (0x7DF2_169C, 4_usize),
        ]);

        let hint = infer_kind_from_resources(&type_counts);
        assert_eq!(hint.kind_hint.as_deref(), Some("Gameplay"));
        assert_eq!(hint.subtype_hint.as_deref(), Some("Gameplay"));
        assert!(hint.confidence_floor >= 0.58);
    }

    #[test]
    fn repeated_gameplay_resource_counts_can_promote_unknown_packages() {
        let type_counts =
            BTreeMap::from([(RESOURCE_STRING_TABLE, 17_usize), (0xE882_D22F, 6_usize)]);

        let hint = infer_kind_from_resources(&type_counts);
        assert_eq!(hint.kind_hint.as_deref(), Some("Gameplay"));
        assert_eq!(hint.subtype_hint.as_deref(), Some("Gameplay"));
        assert!(hint.confidence_floor >= 0.58);
    }

    #[test]
    fn paired_gameplay_resource_types_can_promote_unknown_packages() {
        let type_counts = BTreeMap::from([(0xE882_D22F, 1_usize), (0xEC6A_8FC6, 1_usize)]);

        let hint = infer_kind_from_resources(&type_counts);
        assert_eq!(hint.kind_hint.as_deref(), Some("Gameplay"));
        assert_eq!(hint.subtype_hint.as_deref(), Some("Gameplay"));
        assert!(hint.confidence_floor >= 0.58);
    }

    #[test]
    fn string_support_packages_in_strings_paths_hint_gameplay_utilities() {
        let type_counts = BTreeMap::from([(RESOURCE_STRING_TABLE, 18_usize)]);

        let hint = infer_kind_from_package_signals(
            Path::new("C:/Mods/llazyneiph_royalty_mod/Strings/STRINGS_Coronation.package"),
            &type_counts,
        );
        assert_eq!(hint.kind_hint.as_deref(), Some("Gameplay"));
        assert_eq!(hint.subtype_hint.as_deref(), Some("Utilities"));
        assert!(hint.confidence_floor >= 0.58);
    }

    #[test]
    fn string_support_packages_can_match_filename_tokens_without_folder_help() {
        let type_counts = BTreeMap::from([(RESOURCE_STRING_TABLE, 18_usize)]);

        let hint = infer_kind_from_package_signals(
            Path::new("C:/Mods/Marquillo_SBO_Strings.package"),
            &type_counts,
        );
        assert_eq!(hint.kind_hint.as_deref(), Some("Gameplay"));
        assert_eq!(hint.subtype_hint.as_deref(), Some("Utilities"));
        assert!(hint.confidence_floor >= 0.58);
    }

    #[test]
    fn support_focused_string_packages_allow_small_helper_resource_mix() {
        let type_counts = BTreeMap::from([
            (RESOURCE_STRING_TABLE, 18_usize),
            (0x00B2_D882, 124_usize),
            (0x3C2A_8647, 24_usize),
        ]);

        let hint = infer_kind_from_package_signals(
            Path::new("C:/Mods/[xosdr] Road to Wealth v2.2/xosdr_RTW_M1_Strings.package"),
            &type_counts,
        );
        assert_eq!(hint.kind_hint.as_deref(), Some("Gameplay"));
        assert_eq!(hint.subtype_hint.as_deref(), Some("Utilities"));
        assert!(hint.confidence_floor >= 0.58);
    }

    #[test]
    fn gameplay_context_tokens_can_promote_addon_packages_with_known_gameplay_resources() {
        let type_counts = BTreeMap::from([(0x7DF2_169C, 1_usize)]);

        let hint = infer_kind_from_package_signals(
            Path::new("C:/Mods/AddOns/Andirz_PSO_OnlineStore_Addon_Remove_FromPhone.package"),
            &type_counts,
        );
        assert_eq!(hint.kind_hint.as_deref(), Some("Gameplay"));
        assert_eq!(hint.subtype_hint.as_deref(), Some("Gameplay"));
        assert!(hint.confidence_floor >= 0.56);
    }

    #[test]
    fn split_main_mod_context_tokens_can_promote_gameplay_packages() {
        let type_counts = BTreeMap::from([(0x7DF2_169C, 1_usize)]);

        let hint = infer_kind_from_package_signals(
            Path::new("C:/Mods/SrslySims_SCCOR-MainMod.package"),
            &type_counts,
        );
        assert_eq!(hint.kind_hint.as_deref(), Some("Gameplay"));
        assert_eq!(hint.subtype_hint.as_deref(), Some("Gameplay"));
        assert!(hint.confidence_floor >= 0.56);
    }

    #[test]
    fn uicheats_context_tokens_can_promote_gameplay_packages() {
        let type_counts = BTreeMap::from([(0x62EC_C59A, 1_usize)]);

        let hint = infer_kind_from_package_signals(
            Path::new("C:/Mods/SimMattically_BetterSimologyPanel_withUICheatsExtension.package"),
            &type_counts,
        );
        assert_eq!(hint.kind_hint.as_deref(), Some("Gameplay"));
        assert_eq!(hint.subtype_hint.as_deref(), Some("Gameplay"));
        assert!(hint.confidence_floor >= 0.56);
    }

    #[test]
    fn ambiguous_single_resource_without_context_stays_unknown() {
        let type_counts = BTreeMap::from([(0x62EC_C59A, 1_usize)]);

        let hint = infer_kind_from_package_signals(
            Path::new("C:/Mods/Colorful_Var_Pink.package"),
            &type_counts,
        );
        assert_eq!(hint.kind_hint, None);
        assert_eq!(hint.subtype_hint, None);
    }

    #[test]
    fn lean_cas_appearance_resources_can_hint_cas_packages() {
        let type_counts = BTreeMap::from([
            (0x015A_1849, 24_usize),
            (0x7FB6_AD8A, 1_usize),
            (0xAC16_FBEC, 5_usize),
        ]);

        let hint = infer_kind_from_resources(&type_counts);
        assert_eq!(hint.kind_hint.as_deref(), Some("CAS"));
        assert!(hint.confidence_floor >= 0.68);
    }

    #[test]
    fn mixed_buildbuy_packages_do_not_take_lean_cas_shortcut() {
        let type_counts = BTreeMap::from([
            (0x015A_1849, 24_usize),
            (0x319E_4F1D, 3_usize),
            (0x01D0_E75D, 6_usize),
            (0xAC16_FBEC, 5_usize),
        ]);

        let hint = infer_kind_from_resources(&type_counts);
        assert_eq!(hint.kind_hint.as_deref(), Some("BuildBuy"));
    }

    #[test]
    fn lot_price_context_can_promote_dense_single_resource_packages() {
        let type_counts = BTreeMap::from([(0x0194_2E2C, 416_usize)]);

        let hint = infer_kind_from_package_signals(
            Path::new(
                "C:/Mods/[xosdr] Road to Wealth v2.2/Open Me (optional)/LotPrices (choose one)/xosdr_RTW_M0_LotPrices_10.package",
            ),
            &type_counts,
        );
        assert_eq!(hint.kind_hint.as_deref(), Some("Gameplay"));
        assert_eq!(hint.subtype_hint.as_deref(), Some("Gameplay"));
        assert!(hint.confidence_floor >= 0.58);
    }
}
