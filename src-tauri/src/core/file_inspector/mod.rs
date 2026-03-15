use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom},
    path::Path,
};

use flate2::read::ZlibDecoder;
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

const RESOURCE_NAME_MAP: u32 = 0x0166_038c;
const RESOURCE_STRING_TABLE: u32 = 0x2205_57da;
const RESOURCE_CAS_PART: u32 = 0x034a_eecb;
const RESOURCE_SKINTONE: u32 = 0x0354_796a;
const RESOURCE_CATALOG: u32 = 0x319e_4f1d;
const RESOURCE_DEFINITION: u32 = 0xc0db_5ae7;
const RESOURCE_HOTSPOT: u32 = 0x8b18_ff6e;
const RESOURCE_SCRIPT: u32 = 0x073f_aa07;

#[derive(Debug, Clone, Default)]
pub struct InspectionOutcome {
    pub insights: FileInsights,
    pub creator_hint: Option<String>,
    pub kind_hint: Option<String>,
    pub subtype_hint: Option<String>,
    pub confidence_boost: f64,
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
    let mut stems = BTreeSet::new();
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
        for segment in entry_name.split('/').take(2) {
            let cleaned = segment.trim();
            if !cleaned.is_empty() {
                namespaces.insert(cleaned.to_owned());
            }
        }

        if let Some(filename) = entry_name.rsplit('/').next() {
            let stem = filename.split('.').next().unwrap_or(filename).trim();
            if !stem.is_empty() {
                stems.insert(stem.to_owned());
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
        .chain(stems.iter())
        .chain(payload_identity_values.iter())
        .map(String::as_str)
        .collect::<Vec<_>>();
    let creator_hints = collect_creator_hints(raw_identity_values.iter().copied(), seed_pack);
    let version_signals = collect_ts4script_version_signals(
        path,
        stems.iter().map(String::as_str),
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
        script_namespaces = stems.iter().take(MAX_DISPLAY_VALUES).cloned().collect();
    }

    let primary_creator = creator_hints.first().cloned();
    let mut embedded_names = stems.into_iter().collect::<Vec<_>>();
    embedded_names.extend(payload_identity_values);
    embedded_names = unique_display_values(embedded_names);

    Ok(InspectionOutcome {
        insights: FileInsights {
            format: Some("ts4script-zip".to_owned()),
            resource_summary: vec![
                format!("Archive entries: {}", archive.len()),
                format!("Top-level namespaces: {}", script_namespaces.len()),
            ],
            script_namespaces,
            embedded_names,
            creator_hints: creator_hints.clone(),
            version_hints,
            version_signals,
            family_hints,
        },
        creator_hint: primary_creator,
        kind_hint: Some("ScriptMods".to_owned()),
        subtype_hint: Some("Utilities".to_owned()),
        confidence_boost: 0.14,
    })
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
    let (kind_hint, subtype_hint) = infer_kind_from_resources(&type_counts);

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
        },
        creator_hint: creator_hints.first().cloned(),
        kind_hint,
        subtype_hint,
        confidence_boost: 0.12,
    })
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
    for key in ["name", "mod_name", "mod-name", "modName", "title"] {
        if let Some(value) = json.get(key).and_then(|candidate| candidate.as_str()) {
            if let Some(cleaned) = clean_payload_identity_value(value) {
                values.push(cleaned);
            }
        }
    }

    values
}

fn extract_ts4script_line_identity_values(payload: &str) -> Vec<String> {
    let mut values = Vec::new();

    for line in payload.lines().take(64) {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };

        let lowered_key = key.trim().to_ascii_lowercase();
        if !matches!(
            lowered_key.as_str(),
            "name" | "mod_name" | "mod-name" | "modname" | "title"
        ) {
            continue;
        }

        if let Some(cleaned) = clean_payload_identity_value(value) {
            values.push(cleaned);
        }
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

fn build_resource_summary(
    type_counts: &BTreeMap<u32, usize>,
    records: &[DbpfRecord],
) -> Vec<String> {
    let mut summary = type_counts
        .iter()
        .map(
            |(resource_type, count)| match resource_label(*resource_type) {
                Some(label) => format!("{label} x{count}"),
                None => format!("0x{resource_type:08X} x{count}"),
            },
        )
        .collect::<Vec<_>>();

    let compressed_count = records
        .iter()
        .filter(|record| record.is_compressed())
        .count();
    if compressed_count > 0 {
        summary.push(format!("Compressed resources present: {compressed_count}"));
    }

    summary.truncate(MAX_DISPLAY_VALUES);
    summary
}

fn infer_kind_from_resources(
    type_counts: &BTreeMap<u32, usize>,
) -> (Option<String>, Option<String>) {
    let cas_score = type_counts
        .get(&RESOURCE_CAS_PART)
        .copied()
        .unwrap_or_default()
        + type_counts
            .get(&RESOURCE_SKINTONE)
            .copied()
            .unwrap_or_default();
    let build_buy_score = type_counts
        .get(&RESOURCE_CATALOG)
        .copied()
        .unwrap_or_default()
        + type_counts
            .get(&RESOURCE_DEFINITION)
            .copied()
            .unwrap_or_default();
    let preset_score = type_counts
        .get(&RESOURCE_HOTSPOT)
        .copied()
        .unwrap_or_default();
    let script_score = type_counts
        .get(&RESOURCE_SCRIPT)
        .copied()
        .unwrap_or_default();

    let mut candidates = vec![
        (
            "CAS",
            cas_score,
            if type_counts.contains_key(&RESOURCE_SKINTONE) {
                Some("Skin")
            } else {
                None
            },
        ),
        ("BuildBuy", build_buy_score, None),
        ("PresetsAndSliders", preset_score, Some("Sliders")),
        ("ScriptMods", script_score, Some("Utilities")),
    ];
    candidates.sort_by(|left, right| right.1.cmp(&left.1));

    match candidates.first() {
        Some((kind, score, subtype)) if *score > 0 => (
            Some((*kind).to_owned()),
            subtype.map(|value| value.to_owned()),
        ),
        _ => (None, None),
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
            } else {
                fallback_hints.push(creator);
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
    use std::{fs, fs::File, io::Write};

    use flate2::{write::ZlibEncoder, Compression};
    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;

    use crate::seed::load_seed_pack;

    use super::{
        decompress_legacy, decompress_record_bytes, inspect_file, parse_name_map_entries,
        parse_stbl_entries, read_seven_bit_string_be, DbpfRecord, RESOURCE_STRING_TABLE,
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
}
