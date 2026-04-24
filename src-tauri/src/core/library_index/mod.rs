use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::Path,
};

use crate::{
    core::{content_versions, scanner},
    error::AppResult,
    models::{
        CategoryOverrideInfo, CreatorLearningInfo, FileDetail, FileInsights, FolderTreeMetadata,
        FolderTreeNode, HomeOverview, LibraryFacets, LibraryFileRow, LibraryListResponse,
        LibraryQuery, LibrarySettings, LibrarySortField, LibrarySummary, LibraryWatchFilter,
        WatchStatus,
    },
    seed::{SeedPack, TaxonomySeed},
};

pub fn get_home_overview(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
) -> AppResult<HomeOverview> {
    let total_files = scalar(connection, "SELECT COUNT(*) FROM files")?;
    let mods_count = scalar(
        connection,
        "SELECT COUNT(*) FROM files WHERE kind NOT LIKE 'Tray%'",
    )?;
    let tray_count = scalar(
        connection,
        "SELECT COUNT(*) FROM files WHERE kind LIKE 'Tray%'",
    )?;
    let downloads_count = scalar(
        connection,
        "SELECT COUNT(*) FROM files WHERE source_location = 'downloads'",
    )?;
    let script_mods_count = scalar(
        connection,
        "SELECT COUNT(*) FROM files WHERE kind = 'ScriptMods'",
    )?;
    let creator_count = scalar(
        connection,
        "SELECT COUNT(DISTINCT creator_id) FROM files WHERE creator_id IS NOT NULL",
    )?;
    let bundles_count = scalar(connection, "SELECT COUNT(*) FROM bundles")?;
    let duplicates_count = scalar(connection, "SELECT COUNT(*) FROM duplicates")?;
    let review_count = scalar(connection, "SELECT COUNT(*) FROM review_queue")?;
    let unsafe_count = scalar(
        connection,
        "SELECT COUNT(*) FROM files WHERE safety_notes <> '[]'",
    )?;
    let silent_special_mod_updates = {
        let val = connection
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'silent_special_mod_updates'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok();
        val.as_deref() == Some("true")
    };
    let (exact_update_items, possible_update_items, unknown_watch_items) =
        content_versions::load_watch_counts(connection, silent_special_mod_updates)?;
    let watch_review_items =
        content_versions::list_library_watch_review_items(connection, settings, seed_pack, 1)?
            .total;
    let watch_setup_items =
        content_versions::list_library_watch_setup_items(connection, settings, seed_pack, 1)?.total;
    let last_scan_at = connection
        .query_row(
            "SELECT completed_at
             FROM scan_sessions
             WHERE completed_at IS NOT NULL
             ORDER BY started_at DESC
             LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let scan_needs_refresh = scanner::library_scan_needs_refresh(connection, seed_pack)?;

    Ok(HomeOverview {
        total_files,
        mods_count,
        tray_count,
        downloads_count,
        script_mods_count,
        creator_count,
        bundles_count,
        duplicates_count,
        review_count,
        unsafe_count,
        exact_update_items,
        possible_update_items,
        unknown_watch_items,
        watch_review_items,
        watch_setup_items,
        last_scan_at,
        scan_needs_refresh,
        read_only_mode: true,
    })
}

pub fn get_library_facets(
    connection: &Connection,
    taxonomy: &TaxonomySeed,
    kind_filter: Option<&str>,
) -> AppResult<LibraryFacets> {
    let creators = string_list(
        connection,
        "SELECT DISTINCT c.canonical_name
         FROM files f
         JOIN creators c ON f.creator_id = c.id
         WHERE f.source_location <> 'downloads'
         ORDER BY c.canonical_name COLLATE NOCASE",
    )?;
    let kinds = string_list(
        connection,
        "SELECT DISTINCT kind
         FROM files
         WHERE source_location <> 'downloads'
         ORDER BY kind COLLATE NOCASE",
    )?;
    // Subtypes are scoped to the selected kind when kind_filter is provided.
    // This makes the subtype chips in the UI truthful — they only show subtypes
    // that actually exist on files of the selected kind.
    let subtypes = if let Some(kind) = kind_filter {
        // Parameterized query — kind is bound as ?1
        let sql = "SELECT DISTINCT subtype
                   FROM files
                   WHERE source_location <> 'downloads'
                     AND kind = ?1
                     AND subtype IS NOT NULL
                     AND subtype <> ''
                   ORDER BY subtype COLLATE NOCASE";
        let mut stmt = connection.prepare(sql)?;
        let items = stmt
            .query_map([kind], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()
            .map_err(crate::error::AppError::from)?;
        items
    } else {
        string_list(
            connection,
            "SELECT DISTINCT subtype
             FROM files
             WHERE source_location <> 'downloads'
               AND subtype IS NOT NULL
               AND subtype <> ''
             ORDER BY subtype COLLATE NOCASE",
        )?
    };
    let sources = string_list(
        connection,
        "SELECT DISTINCT source_location
         FROM files
         WHERE source_location <> 'downloads'
         ORDER BY source_location COLLATE NOCASE",
    )?;

    Ok(LibraryFacets {
        creators,
        kinds,
        subtypes,
        sources,
        taxonomy_kinds: taxonomy
            .kinds
            .iter()
            .chain(taxonomy.tray_kinds.iter())
            .cloned()
            .collect(),
    })
}

/// Returns summary counts for the Library strip, filtered to installed content only.
pub fn get_library_summary(connection: &Connection) -> AppResult<LibrarySummary> {
    let total = scalar(
        connection,
        "SELECT COUNT(*) FROM files WHERE source_location <> 'downloads'",
    )?;

    let tracked = scalar(
        connection,
        "SELECT COUNT(DISTINCT f.id)\
         FROM files f\
         JOIN content_watch_sources cws ON cws.anchor_file_id = f.id\
         WHERE f.source_location <> 'downloads'",
    )?;

    let not_tracked = scalar(
        connection,
        "SELECT COUNT(*)\
         FROM files f\
         LEFT JOIN content_watch_sources cws ON cws.anchor_file_id = f.id\
         WHERE f.source_location <> 'downloads' AND cws.subject_key IS NULL",
    )?;

    let has_updates = scalar(
        connection,
        "SELECT COUNT(DISTINCT f.id)\
         FROM files f\
         JOIN content_watch_sources cws ON cws.anchor_file_id = f.id\
         JOIN content_watch_results cwr ON cwr.subject_key = cws.subject_key\
         WHERE f.source_location <> 'downloads'\
         AND cwr.status IN ('exact_update_available', 'possible_update')",
    )?;

    let needs_review = scalar(
        connection,
        "SELECT COUNT(*)\
         FROM files f\
         WHERE f.source_location <> 'downloads'\
         AND (f.safety_notes <> '[]' OR f.parser_warnings <> '[]')",
    )?;

    let duplicates = scalar(
        connection,
        "SELECT COUNT(DISTINCT d.file_id_a)\
         FROM duplicates d",
    )?;

    let disabled = scalar(
        connection,
        "SELECT COUNT(*) FROM files WHERE source_location = 'tray'",
    )?;

    Ok(LibrarySummary {
        total: total as i64,
        tracked: tracked as i64,
        not_tracked: not_tracked as i64,
        has_updates: has_updates as i64,
        needs_review: needs_review as i64,
        duplicates: duplicates as i64,
        disabled: disabled as i64,
    })
}

pub fn list_library_files(
    connection: &Connection,
    query: LibraryQuery,
) -> AppResult<LibraryListResponse> {
    let include_previews = query.include_previews.unwrap_or(true);
    let compact_paged_rows = query.limit.is_some();
    let (filters, params) = build_filters(&query);
    let order_by = build_order_by(query.sort_by);

    let total_sql = format!(
        "SELECT COUNT(*)\n\
         FROM files f\n\
         LEFT JOIN creators c ON f.creator_id = c.id\n\
         LEFT JOIN bundles b ON f.bundle_id = b.id\n\
         LEFT JOIN content_watch_sources cws ON cws.anchor_file_id = f.id\n\
         LEFT JOIN content_watch_results cwr ON cwr.subject_key = cws.subject_key\n\
         WHERE f.source_location <> 'downloads'\n\
        {filters}",
        filters = filters
    );

    let total = connection.query_row(&total_sql, params_from_iter(params.iter()), |row| {
        row.get(0)
    })?;
    let peer_counts = load_relationship_peer_counts(connection, &filters, &params)?;

    // Pagination: only apply LIMIT/OFFSET when query.limit is explicitly set.
    // When limit is None (tree-mode), return all filtered rows without LIMIT/OFFSET.
    let rows_sql = if query.limit.is_some() {
        let limit = query.limit.unwrap_or(100);
        let offset = query.offset.unwrap_or(0);
        let mut row_params = params.clone();
        row_params.push(Value::Integer(limit));
        row_params.push(Value::Integer(offset));
        format!(
            "SELECT\n\
             f.id,\n\
             f.filename,\n\
             f.path,\n\
             f.extension,\n\
             f.kind,\n\
             f.subtype,\n\
             f.confidence,\n\
             f.source_location,\n\
             f.size,\n\
             f.modified_at,\n\
             c.canonical_name,\n\
             b.bundle_name,\n\
             b.bundle_type,\n\
             b.file_count,\n\
             f.relative_depth,\n\
             f.safety_notes,\n\
             f.parser_warnings,\n\
             f.insights,\n\
             cwr.status,\n\
             EXISTS (\n\
               SELECT 1 FROM duplicates d\n\
               WHERE d.file_id_a = f.id OR d.file_id_b = f.id\n\
             ) AS has_duplicate,
\
             0 AS same_folder_peer_count,
\
             0 AS same_pack_peer_count
\
             FROM files f\n\
             LEFT JOIN creators c ON f.creator_id = c.id\n\
             LEFT JOIN bundles b ON f.bundle_id = b.id\n\
             LEFT JOIN content_watch_sources cws ON cws.anchor_file_id = f.id\n\
             LEFT JOIN content_watch_results cwr ON cwr.subject_key = cws.subject_key\n\
             WHERE f.source_location <> 'downloads'\n\
            {filters}\n\
             {order_by}\n\
             LIMIT ? OFFSET ?",
            filters = filters,
            order_by = order_by
        )
    } else {
        format!(
            "SELECT\n\
             f.id,\n\
             f.filename,\n\
             f.path,\n\
             f.extension,\n\
             f.kind,\n\
             f.subtype,\n\
             f.confidence,\n\
             f.source_location,\n\
             f.size,\n\
             f.modified_at,\n\
             c.canonical_name,\n\
             b.bundle_name,\n\
             b.bundle_type,\n\
             b.file_count,\n\
             f.relative_depth,\n\
             f.safety_notes,\n\
             f.parser_warnings,\n\
             f.insights,\n\
             cwr.status,\n\
             EXISTS (\n\
               SELECT 1 FROM duplicates d\n\
               WHERE d.file_id_a = f.id OR d.file_id_b = f.id\n\
             ) AS has_duplicate,
\
             0 AS same_folder_peer_count,
\
             0 AS same_pack_peer_count
\
             FROM files f\n\
             LEFT JOIN creators c ON f.creator_id = c.id\n\
             LEFT JOIN bundles b ON f.bundle_id = b.id\n\
             LEFT JOIN content_watch_sources cws ON cws.anchor_file_id = f.id\n\
             LEFT JOIN content_watch_results cwr ON cwr.subject_key = cws.subject_key\n\
             WHERE f.source_location <> 'downloads'\n\
            {filters}\n\
             {order_by}",
            filters = filters,
            order_by = order_by
        )
    };

    let row_params = if query.limit.is_some() {
        let limit = query.limit.unwrap_or(100);
        let offset = query.offset.unwrap_or(0);
        let mut p = params.clone();
        p.push(Value::Integer(limit));
        p.push(Value::Integer(offset));
        p
    } else {
        params.clone()
    };

    let mut statement = connection.prepare(&rows_sql)?;
    let items = statement
        .query_map(params_from_iter(row_params.iter()), |row| {
            let id = row.get::<_, i64>(0)?;
            let (same_folder_peer_count, same_pack_peer_count) =
                peer_counts.get(&id).copied().unwrap_or_default();
            let watch_status_str: Option<String> = row.get(18)?;
            let watch_status = watch_status_str
                .map(|s| match s.as_str() {
                    "current" => WatchStatus::Current,
                    "exact_update_available" => WatchStatus::ExactUpdateAvailable,
                    "possible_update" => WatchStatus::PossibleUpdate,
                    "unknown" => WatchStatus::Unknown,
                    _ => WatchStatus::NotWatched,
                })
                .unwrap_or_default();
            Ok(LibraryFileRow {
                id,
                filename: row.get(1)?,
                path: row.get(2)?,
                extension: row.get(3)?,
                kind: row.get(4)?,
                subtype: row.get(5)?,
                confidence: row.get(6)?,
                source_location: row.get(7)?,
                size: row.get(8)?,
                modified_at: row.get(9)?,
                creator: row.get(10)?,
                bundle_name: row.get(11)?,
                bundle_type: row.get(12)?,
                grouped_file_count: row.get(13)?,
                relative_depth: row.get(14)?,
                safety_notes: parse_string_array(row.get::<_, String>(15)?),
                parser_warnings: parse_string_array(row.get::<_, String>(16)?),
                insights: compact_library_row_insights(
                    parse_insights(row.get::<_, Option<String>>(17)?),
                    include_previews,
                    compact_paged_rows,
                ),
                watch_status,
                has_duplicate: row.get::<_, i64>(19)? != 0,
                installed_version: None,
                same_folder_peer_count,
                same_pack_peer_count,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(LibraryListResponse { total, items })
}

#[derive(Debug)]
struct RelationshipPeerRow {
    id: i64,
    path: String,
    source_location: String,
    relative_depth: i64,
    bundle_id: Option<i64>,
}

fn load_relationship_peer_counts(
    connection: &Connection,
    filters: &str,
    params: &[Value],
) -> AppResult<HashMap<i64, (i64, i64)>> {
    let sql = format!(
        "SELECT DISTINCT f.id, f.path, f.source_location, f.relative_depth, f.bundle_id\n\
         FROM files f\n\
         LEFT JOIN creators c ON f.creator_id = c.id\n\
         LEFT JOIN content_watch_sources cws ON cws.anchor_file_id = f.id\n\
         LEFT JOIN content_watch_results cwr ON cwr.subject_key = cws.subject_key\n\
         WHERE f.source_location <> 'downloads'\n\
        {filters}",
        filters = filters
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement
        .query_map(params_from_iter(params.iter()), |row| {
            Ok(RelationshipPeerRow {
                id: row.get(0)?,
                path: row.get(1)?,
                source_location: row.get(2)?,
                relative_depth: row.get(3)?,
                bundle_id: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut folder_groups: HashMap<(String, String), Vec<i64>> = HashMap::new();
    let mut bundle_groups: HashMap<i64, Vec<i64>> = HashMap::new();

    for row in rows {
        if let Some(folder_segments) =
            folder_segments_for_file(&row.path, &row.source_location, row.relative_depth)
        {
            folder_groups
                .entry((
                    row.source_location.to_ascii_lowercase(),
                    folder_segments.join("/"),
                ))
                .or_default()
                .push(row.id);
        }

        if let Some(bundle_id) = row.bundle_id {
            bundle_groups.entry(bundle_id).or_default().push(row.id);
        }
    }

    let mut counts = HashMap::new();
    for ids in folder_groups.values() {
        let peer_count = ids.len().saturating_sub(1) as i64;
        for id in ids {
            counts.entry(*id).or_insert((0, 0)).0 = peer_count;
        }
    }
    for ids in bundle_groups.values() {
        let peer_count = ids.len().saturating_sub(1) as i64;
        for id in ids {
            counts.entry(*id).or_insert((0, 0)).1 = peer_count;
        }
    }

    Ok(counts)
}

#[derive(Debug)]
struct FolderFileRow {
    path: String,
    source_location: String,
    relative_depth: i64,
}

#[derive(Debug, Clone)]
struct FolderNodeAccumulator {
    path: String,
    name: String,
    depth: i64,
    source_location: String,
    direct_file_count: i64,
    total_file_count: i64,
    children: BTreeSet<String>,
}

pub fn get_folder_tree_metadata(
    connection: &Connection,
    query: &LibraryQuery,
) -> AppResult<FolderTreeMetadata> {
    let (filters, params) = build_filters(query);
    let sql = format!(
        "SELECT f.path, f.source_location, f.relative_depth\n\
         FROM files f\n\
         LEFT JOIN creators c ON f.creator_id = c.id\n\
         LEFT JOIN content_watch_sources cws ON cws.anchor_file_id = f.id\n\
         LEFT JOIN content_watch_results cwr ON cwr.subject_key = cws.subject_key\n\
         WHERE f.source_location <> 'downloads'\n\
        {filters}\n\
         ORDER BY f.source_location COLLATE NOCASE, f.path COLLATE NOCASE",
        filters = filters
    );

    let mut statement = connection.prepare(&sql)?;
    let rows = statement
        .query_map(params_from_iter(params.iter()), |row| {
            Ok(FolderFileRow {
                path: row.get(0)?,
                source_location: row.get(1)?,
                relative_depth: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(build_folder_metadata_from_rows(rows))
}

fn build_folder_metadata_from_rows(rows: Vec<FolderFileRow>) -> FolderTreeMetadata {
    let mut nodes: BTreeMap<String, FolderNodeAccumulator> = BTreeMap::new();

    for row in rows {
        let Some(segments) =
            folder_segments_for_file(&row.path, &row.source_location, row.relative_depth)
        else {
            continue;
        };

        for index in 0..segments.len() {
            let path = segments[..=index].join("/");
            let source_location = row.source_location.to_ascii_lowercase();
            let entry = nodes
                .entry(path.clone())
                .or_insert_with(|| FolderNodeAccumulator {
                    path: path.clone(),
                    name: segments[index].clone(),
                    depth: index as i64,
                    source_location,
                    direct_file_count: 0,
                    total_file_count: 0,
                    children: BTreeSet::new(),
                });
            entry.total_file_count += 1;
            if index == segments.len() - 1 {
                entry.direct_file_count += 1;
            }

            if index > 0 {
                let parent_path = segments[..index].join("/");
                if let Some(parent) = nodes.get_mut(&parent_path) {
                    parent.children.insert(path.clone());
                }
            }
        }
    }

    let mut roots = nodes
        .values()
        .filter(|node| node.depth == 0)
        .map(|node| build_folder_node(&node.path, &nodes))
        .collect::<Vec<_>>();
    roots.sort_by(compare_folder_nodes);

    let total_folders = roots
        .iter()
        .map(|root| 1 + count_folder_descendants(&root.children))
        .sum();

    FolderTreeMetadata {
        total_folders,
        roots,
    }
}

fn build_folder_node(
    path: &str,
    nodes: &BTreeMap<String, FolderNodeAccumulator>,
) -> FolderTreeNode {
    let node = nodes.get(path).expect("folder node exists");
    let mut children = node
        .children
        .iter()
        .map(|child_path| build_folder_node(child_path, nodes))
        .collect::<Vec<_>>();
    children.sort_by(compare_folder_nodes);

    FolderTreeNode {
        path: node.path.clone(),
        name: node.name.clone(),
        depth: node.depth,
        source_location: node.source_location.clone(),
        direct_file_count: node.direct_file_count,
        child_folder_count: children.len() as i64,
        total_file_count: node.total_file_count,
        children,
    }
}

fn count_folder_descendants(nodes: &[FolderTreeNode]) -> i64 {
    nodes
        .iter()
        .map(|node| 1 + count_folder_descendants(&node.children))
        .sum()
}

fn compare_folder_nodes(left: &FolderTreeNode, right: &FolderTreeNode) -> std::cmp::Ordering {
    folder_sort_rank(&left.name)
        .cmp(&folder_sort_rank(&right.name))
        .then_with(|| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
        })
}

fn folder_sort_rank(name: &str) -> u8 {
    match name {
        "Mods" => 0,
        "Tray" => 1,
        _ => 2,
    }
}

fn folder_segments_for_file(
    path: &str,
    source_location: &str,
    relative_depth: i64,
) -> Option<Vec<String>> {
    let root = folder_root_name(source_location)?;
    let normalized = path.replace('\\', "/");
    let components = normalized
        .split('/')
        .filter(|part| !part.is_empty())
        .map(|part| part.trim().to_owned())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let parent_components = components
        .len()
        .checked_sub(1)
        .map(|last| components[..last].to_vec())
        .unwrap_or_default();
    let folder_depth = relative_depth.max(0) as usize;

    let child_segments = if folder_depth > 0 && parent_components.len() >= folder_depth {
        parent_components[parent_components.len() - folder_depth..].to_vec()
    } else {
        Vec::new()
    };

    let mut segments = Vec::with_capacity(child_segments.len() + 1);
    segments.push(root);
    segments.extend(child_segments);
    Some(segments)
}

fn folder_root_name(source_location: &str) -> Option<String> {
    let trimmed = source_location.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("downloads") {
        return None;
    }

    if trimmed.eq_ignore_ascii_case("mods") {
        return Some("Mods".to_owned());
    }
    if trimmed.eq_ignore_ascii_case("tray") {
        return Some("Tray".to_owned());
    }

    let mut chars = trimmed.chars();
    let Some(first) = chars.next() else {
        return None;
    };
    Some(format!(
        "{}{}",
        first.to_ascii_uppercase(),
        chars.as_str().to_ascii_lowercase()
    ))
}

pub fn get_file_detail(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    file_id: i64,
) -> AppResult<Option<FileDetail>> {
    let detail = connection
        .query_row(
            "SELECT
                f.id,
                f.filename,
                f.path,
                f.extension,
                f.kind,
                f.subtype,
                f.confidence,
                f.source_location,
                f.size,
                f.modified_at,
                c.canonical_name,
                b.bundle_name,
                b.bundle_type,
                b.file_count,
                f.relative_depth,
                f.safety_notes,
                f.hash,
                f.created_at,
                f.parser_warnings,
                f.insights,
                c.id,
                COALESCE(c.locked_by_user, 0),
                c.preferred_path,
                uco.kind,
                uco.subtype
             FROM files f
             LEFT JOIN creators c ON f.creator_id = c.id
             LEFT JOIN bundles b ON f.bundle_id = b.id
             LEFT JOIN user_category_overrides uco ON uco.match_path = f.path
             WHERE f.id = ?1
               AND f.source_location <> 'downloads'",
            params![file_id],
            |row| {
                Ok(FileDetail {
                    id: row.get(0)?,
                    filename: row.get(1)?,
                    path: row.get(2)?,
                    extension: row.get(3)?,
                    kind: row.get(4)?,
                    subtype: row.get(5)?,
                    confidence: row.get(6)?,
                    source_location: row.get(7)?,
                    size: row.get(8)?,
                    modified_at: row.get(9)?,
                    creator: row.get(10)?,
                    bundle_name: row.get(11)?,
                    bundle_type: row.get(12)?,
                    grouped_file_count: row.get(13)?,
                    relative_depth: row.get(14)?,
                    safety_notes: parse_string_array(row.get::<_, String>(15)?),
                    hash: row.get(16)?,
                    created_at: row.get(17)?,
                    parser_warnings: parse_string_array(row.get::<_, String>(18)?),
                    insights: parse_insights(Some(row.get::<_, String>(19)?)),
                    installed_version_summary: None,
                    watch_result: None,
                    creator_learning: CreatorLearningInfo {
                        locked_by_user: row.get::<_, i64>(21)? != 0,
                        preferred_path: row.get(22)?,
                        learned_aliases: Vec::new(),
                    },
                    category_override: {
                        let kind: Option<String> = row.get(23)?;
                        CategoryOverrideInfo {
                            saved_by_user: kind.is_some(),
                            kind,
                            subtype: row.get(24)?,
                        }
                    },
                    duplicates_count: 0,
                    duplicate_types: Vec::new(),
                    installed_version: None,
                })
            },
        )
        .optional()?;

    match detail {
        Some(mut detail) => {
            if let Some(creator_name) = detail.creator.as_deref() {
                detail.creator_learning.learned_aliases =
                    list_creator_aliases(connection, creator_name)?;
            }
            let (installed_version_summary, watch_result) =
                content_versions::resolve_library_file_version(
                    connection, settings, seed_pack, file_id,
                )?;
            detail.installed_version_summary = installed_version_summary;
            detail.watch_result = watch_result;

            // Load duplicate info for this file.
            let duplicate_types: Vec<String> = connection
                .prepare(
                    "SELECT DISTINCT duplicate_type FROM duplicates
                     WHERE file_id_a = ?1 OR file_id_b = ?1
                     ORDER BY duplicate_type",
                )?
                .query_map(params![file_id], |row| row.get(0))?
                .collect::<Result<Vec<String>, _>>()?;
            detail.duplicates_count = duplicate_types.len();
            detail.duplicate_types = duplicate_types;

            // Phase 5an: resolve thumbnails on-demand if they were deferred during scan.
            // During scan, thumbnails are skipped (THUMBNAIL_DEFERRED=true) to avoid
            // 3× DBPF re-parse per file. Here we do the deferred thumbnail work.
            use crate::core::file_inspector::resolve_package_thumbnails_deferred;
            let (embedded_thumb, cached_thumb) =
                resolve_package_thumbnails_deferred(Path::new(&detail.path));
            detail.insights.thumbnail_preview =
                detail.insights.thumbnail_preview.or(embedded_thumb);
            detail.insights.cached_thumbnail_preview =
                detail.insights.cached_thumbnail_preview.or(cached_thumb);

            Ok(Some(detail))
        }
        None => Ok(None),
    }
}

pub fn build_filters(query: &LibraryQuery) -> (String, Vec<Value>) {
    let mut sql = String::new();
    let mut params = Vec::new();

    if let Some(search) = query
        .search
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        sql.push_str(" AND (f.filename LIKE ? OR f.path LIKE ? OR COALESCE(c.canonical_name, '') LIKE ? OR COALESCE(f.subtype, '') LIKE ?)");
        let pattern = format!("%{search}%");
        params.push(Value::Text(pattern.clone()));
        params.push(Value::Text(pattern.clone()));
        params.push(Value::Text(pattern.clone()));
        params.push(Value::Text(pattern));
    }

    if let Some(kind) = query.kind.as_ref().filter(|value| !value.is_empty()) {
        sql.push_str(" AND f.kind = ?");
        params.push(Value::Text(kind.clone()));
    }

    if let Some(subtype) = query.subtype.as_ref().filter(|value| !value.is_empty()) {
        sql.push_str(" AND f.subtype = ?");
        params.push(Value::Text(subtype.clone()));
    }

    if let Some(creator) = query.creator.as_ref().filter(|value| !value.is_empty()) {
        sql.push_str(" AND c.canonical_name = ?");
        params.push(Value::Text(creator.clone()));
    }

    if let Some(source) = query.source.as_ref().filter(|value| !value.is_empty()) {
        sql.push_str(" AND f.source_location = ?");
        params.push(Value::Text(source.clone()));
    }

    if let Some(min_confidence) = query.min_confidence {
        sql.push_str(" AND f.confidence >= ?");
        params.push(Value::Real(min_confidence));
    }

    // Apply watch-state quick filter. Relies on cws/cwr JOIN being present.
    match query.watch_filter.unwrap_or_default() {
        LibraryWatchFilter::HasUpdates => {
            sql.push_str(" AND cwr.status IN ('exact_update_available', 'possible_update')");
        }
        LibraryWatchFilter::NeedsAttention => {
            // Includes both safety notes (genuine concerns) and parser warnings
            // (uncertain metadata) — items the user should manually review.
            sql.push_str(" AND (f.safety_notes <> '[]' OR f.parser_warnings <> '[]')");
        }
        LibraryWatchFilter::NotTracked => {
            sql.push_str(" AND cws.subject_key IS NULL");
        }
        LibraryWatchFilter::Duplicates => {
            sql.push_str(
                " AND EXISTS (\
                 SELECT 1 FROM duplicates d\
                 WHERE d.file_id_a = f.id OR d.file_id_b = f.id)",
            );
        }
        LibraryWatchFilter::All => {}
    }

    (sql, params)
}

fn build_order_by(sort_by: Option<LibrarySortField>) -> String {
    match sort_by.unwrap_or_default() {
        LibrarySortField::Name => String::from("ORDER BY f.filename COLLATE NOCASE"),
        LibrarySortField::Creator => String::from(
            "ORDER BY c.canonical_name COLLATE NOCASE ASC, f.filename COLLATE NOCASE ASC",
        ),
        LibrarySortField::RecentlyModified => String::from(
            "ORDER BY\
                 CASE WHEN f.modified_at IS NULL THEN 1 ELSE 0 END ASC,\
                 f.modified_at DESC,\
                 f.filename COLLATE NOCASE ASC",
        ),
        LibrarySortField::HasUpdatesFirst => {
            // Sort by update priority: exact_update_available first, then possible_update,
            // then unknown, then current, then not_watched. Tie-break by filename.
            String::from(
                "ORDER BY\
                 CASE cwr.status\
                 WHEN 'exact_update_available' THEN 1\
                 WHEN 'possible_update' THEN 2\
                 WHEN 'unknown' THEN 3\
                 WHEN 'current' THEN 4\
                 ELSE 5\
                 END ASC,\
                 f.filename COLLATE NOCASE ASC",
            )
        }
    }
}

fn scalar(connection: &Connection, sql: &str) -> AppResult<i64> {
    connection
        .query_row(sql, [], |row| row.get(0))
        .map_err(Into::into)
}

fn string_list(connection: &Connection, sql: &str) -> AppResult<Vec<String>> {
    let mut statement = connection.prepare(sql)?;
    let items = statement
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()
        .map_err(crate::error::AppError::from)?;
    Ok(items)
}

fn parse_string_array(value: String) -> Vec<String> {
    serde_json::from_str(&value).unwrap_or_default()
}

fn parse_insights(value: Option<String>) -> FileInsights {
    match value {
        Some(v) => serde_json::from_str(&v).unwrap_or_default(),
        None => FileInsights::default(),
    }
}

fn compact_library_row_insights(
    mut insights: FileInsights,
    include_previews: bool,
    compact: bool,
) -> FileInsights {
    if !include_previews {
        insights.thumbnail_preview = None;
        insights.cached_thumbnail_preview = None;
    }

    if compact {
        insights.resource_summary.truncate(1);
        insights.script_namespaces.truncate(3);
        insights.embedded_names.truncate(4);
        insights.creator_hints.truncate(2);
        insights.version_hints.truncate(1);
        insights.version_signals.truncate(1);
        insights.family_hints.truncate(4);
    }

    insights
}

fn list_creator_aliases(connection: &Connection, canonical_name: &str) -> AppResult<Vec<String>> {
    let mut statement = connection.prepare(
        "SELECT u.alias_name
         FROM user_creator_aliases u
         JOIN creators c ON c.id = u.creator_id
         WHERE c.canonical_name = ?1 COLLATE NOCASE
         ORDER BY u.updated_at DESC, u.alias_name COLLATE NOCASE",
    )?;

    let aliases = statement
        .query_map(params![canonical_name], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()
        .map_err(crate::error::AppError::from)?;

    Ok(aliases)
}

#[cfg(test)]
mod tests {
    use rusqlite::params;

    use crate::{
        database,
        models::{LibraryQuery, LibrarySettings},
        seed::load_seed_pack,
    };

    use super::{
        get_file_detail, get_folder_tree_metadata, get_library_facets, list_library_files,
    };

    fn setup_library_env() -> (rusqlite::Connection, LibrarySettings, crate::seed::SeedPack) {
        let mut connection = rusqlite::Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        let seed_pack = load_seed_pack().expect("seed");
        database::seed_database(&mut connection, &seed_pack).expect("seed db");

        let settings = LibrarySettings {
            mods_path: Some("C:/Mods".to_owned()),
            tray_path: Some("C:/Tray".to_owned()),
            downloads_path: Some("C:/Downloads".to_owned()),
            ..Default::default()
        };

        connection
            .execute(
                "INSERT INTO creators (canonical_name, notes) VALUES (?1, ?2)",
                params!["TestCreator", "fixture"],
            )
            .expect("creator");
        let creator_id = connection.last_insert_rowid();
        let insights_json =
            serde_json::to_string(&crate::models::FileInsights::default()).expect("insights json");

        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    creator_id,
                    kind,
                    subtype,
                    confidence,
                    source_location,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    "C:/Mods/TestCreator/installed.package",
                    "installed.package",
                    ".package",
                    creator_id,
                    "Gameplay",
                    "Utility",
                    0.91_f64,
                    "mods",
                    "[]",
                    insights_json,
                ],
            )
            .expect("installed file");
        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    creator_id,
                    kind,
                    subtype,
                    confidence,
                    source_location,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    "C:/Downloads/incoming.package",
                    "incoming.package",
                    ".package",
                    creator_id,
                    "Gameplay",
                    "Utility",
                    0.88_f64,
                    "downloads",
                    "[]",
                    serde_json::to_string(&crate::models::FileInsights::default())
                        .expect("insights json"),
                ],
            )
            .expect("download file");

        (connection, settings, seed_pack)
    }

    #[test]
    fn library_queries_focus_on_installed_content() {
        let (connection, settings, seed_pack) = setup_library_env();

        let listing =
            list_library_files(&connection, LibraryQuery::default()).expect("library listing");
        assert_eq!(listing.total, 1);
        assert_eq!(listing.items.len(), 1);
        assert_eq!(listing.items[0].filename, "installed.package");
        assert_eq!(listing.items[0].source_location, "mods");

        let facets = get_library_facets(&connection, &seed_pack.taxonomy, None).expect("facets");
        assert_eq!(facets.sources, vec!["mods".to_owned()]);
        assert_eq!(facets.creators, vec!["TestCreator".to_owned()]);

        let download_detail =
            get_file_detail(&connection, &settings, &seed_pack, 2).expect("download detail lookup");
        assert!(download_detail.is_none());
    }

    #[test]
    fn library_listing_supports_paged_queries() {
        let (connection, _settings, _seed_pack) = setup_library_env();

        let listing = list_library_files(
            &connection,
            LibraryQuery {
                limit: Some(1),
                offset: Some(0),
                ..Default::default()
            },
        )
        .expect("paged library listing");

        assert_eq!(listing.total, 1);
        assert_eq!(listing.items.len(), 1);
        assert_eq!(listing.items[0].filename, "installed.package");
    }

    #[test]
    fn folder_tree_metadata_builds_nested_counts() {
        let (connection, _settings, _seed_pack) = setup_library_env();
        let insights_json =
            serde_json::to_string(&crate::models::FileInsights::default()).expect("insights json");

        connection
            .execute(
                "UPDATE files SET relative_depth = 1 WHERE filename = 'installed.package'",
                [],
            )
            .expect("align fixture depth");
        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    relative_depth,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    "C:/Mods/root.package",
                    "root.package",
                    ".package",
                    "Gameplay",
                    0.8_f64,
                    "mods",
                    0_i64,
                    "[]",
                    insights_json,
                ],
            )
            .expect("root file");
        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    relative_depth,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    "C:/Mods/TestCreator/Nested/deep.package",
                    "deep.package",
                    ".package",
                    "Gameplay",
                    0.8_f64,
                    "mods",
                    2_i64,
                    "[]",
                    serde_json::to_string(&crate::models::FileInsights::default())
                        .expect("insights json"),
                ],
            )
            .expect("nested file");
        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    relative_depth,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    "C:/Tray/household.trayitem",
                    "household.trayitem",
                    ".trayitem",
                    "TrayHousehold",
                    0.9_f64,
                    "tray",
                    0_i64,
                    "[]",
                    serde_json::to_string(&crate::models::FileInsights::default())
                        .expect("insights json"),
                ],
            )
            .expect("tray file");

        let metadata = get_folder_tree_metadata(&connection, &LibraryQuery::default())
            .expect("folder metadata");

        let mods = metadata
            .roots
            .iter()
            .find(|node| node.name == "Mods")
            .expect("mods root");
        assert_eq!(mods.direct_file_count, 1);
        assert_eq!(mods.total_file_count, 3);
        assert_eq!(mods.child_folder_count, 1);

        let test_creator = mods
            .children
            .iter()
            .find(|node| node.name == "TestCreator")
            .expect("creator folder");
        assert_eq!(test_creator.path, "Mods/TestCreator");
        assert_eq!(test_creator.direct_file_count, 1);
        assert_eq!(test_creator.total_file_count, 2);
        assert_eq!(test_creator.child_folder_count, 1);

        let nested = test_creator
            .children
            .iter()
            .find(|node| node.name == "Nested")
            .expect("nested folder");
        assert_eq!(nested.direct_file_count, 1);
        assert_eq!(nested.total_file_count, 1);

        let tray = metadata
            .roots
            .iter()
            .find(|node| node.name == "Tray")
            .expect("tray root");
        assert_eq!(tray.direct_file_count, 1);
        assert_eq!(tray.total_file_count, 1);
    }

    #[test]
    fn relationship_peer_counts_use_real_parent_folder_and_real_bundle() {
        let (connection, _settings, _seed_pack) = setup_library_env();
        let insights_json =
            serde_json::to_string(&crate::models::FileInsights::default()).expect("insights json");

        connection
            .execute(
                "UPDATE files SET relative_depth = 1 WHERE filename = 'installed.package'",
                [],
            )
            .expect("align fixture depth");
        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    relative_depth,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    "C:/Mods/TestCreator/sibling.package",
                    "sibling.package",
                    ".package",
                    "Gameplay",
                    0.8_f64,
                    "mods",
                    1_i64,
                    "[]",
                    insights_json,
                ],
            )
            .expect("same folder file");
        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    relative_depth,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    "C:/Mods/Other/other.package",
                    "other.package",
                    ".package",
                    "Gameplay",
                    0.8_f64,
                    "mods",
                    1_i64,
                    "[]",
                    serde_json::to_string(&crate::models::FileInsights::default())
                        .expect("insights json"),
                ],
            )
            .expect("different folder file");

        let listing =
            list_library_files(&connection, LibraryQuery::default()).expect("library listing");
        let installed = listing
            .items
            .iter()
            .find(|item| item.filename == "installed.package")
            .expect("installed row");
        let other = listing
            .items
            .iter()
            .find(|item| item.filename == "other.package")
            .expect("other row");

        assert_eq!(installed.same_folder_peer_count, 1);
        assert_eq!(other.same_folder_peer_count, 0);
        assert_eq!(installed.same_pack_peer_count, 0);
        assert_eq!(other.same_pack_peer_count, 0);
    }

    #[test]
    fn facets_and_listing_respect_kind_scoped_subtypes() {
        let (connection, _settings, seed_pack) = setup_library_env();

        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    subtype,
                    confidence,
                    source_location,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    "C:/Mods/BuildBuy/chair.package",
                    "chair.package",
                    ".package",
                    "BuildBuy",
                    "Seating",
                    0.82_f64,
                    "mods",
                    "[]",
                    serde_json::to_string(&crate::models::FileInsights::default())
                        .expect("insights json"),
                ],
            )
            .expect("buildbuy file");

        let facets =
            get_library_facets(&connection, &seed_pack.taxonomy, Some("BuildBuy")).expect("facets");
        assert_eq!(facets.subtypes, vec!["Seating".to_owned()]);

        let listing = list_library_files(
            &connection,
            LibraryQuery {
                kind: Some("BuildBuy".to_owned()),
                subtype: Some("Seating".to_owned()),
                ..Default::default()
            },
        )
        .expect("filtered listing");
        assert_eq!(listing.total, 1);
        assert_eq!(listing.items[0].kind, "BuildBuy");
        assert_eq!(listing.items[0].subtype.as_deref(), Some("Seating"));
    }
}
