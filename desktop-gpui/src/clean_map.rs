//! Read-only disk map of an accepted Clean scan.
//!
//! The map sizes scanned cleanup targets by estimated bytes and colors them by
//! content class. It does not walk the filesystem, invent free space, compose
//! a shell command, or select a target. Selection stays on the workbench.

use std::collections::HashMap;

/// Outer tiles, including the aggregated remainder.
pub const MAX_GROUPS: usize = 12;
/// Leaves inside one group, including that group's aggregated remainder.
pub const MAX_LEAVES: usize = 6;
/// Rows that receive their own parent group. The byte tail stays in Other.
pub const MAX_DETAILED_ROWS: usize = 256;
/// Canvas used by the Found-state map. Layout is independent of the window.
pub const MAP_CANVAS_WIDTH: f32 = 560.0;
pub const MAP_CANVAS_HEIGHT: f32 = 220.0;

const GROUP_HEADER: f64 = 18.0;

/// Content class for one scanned target. Color is not a safety claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MapCategory {
    /// Project build output and dependency directories.
    BuildOutput,
    /// Regenerable tool or package caches.
    SafeToClear,
    /// AI assistant history. Generic clean scan does not emit these ids.
    AgentHistory,
    /// Model or hub caches that are not a normal build directory.
    AiModels,
    /// Docker observations. This build does not prune Docker.
    Docker,
    /// Inspect-only tooling such as Cargo home.
    SystemApps,
    /// A rule this map does not classify, or an aggregated remainder.
    Other,
}

impl MapCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BuildOutput => "build",
            Self::SafeToClear => "cache",
            Self::AgentHistory => "agent",
            Self::AiModels => "models",
            Self::Docker => "docker",
            Self::SystemApps => "system",
            Self::Other => "other",
        }
    }

    const fn rank(self) -> u8 {
        match self {
            Self::BuildOutput => 0,
            Self::SafeToClear => 1,
            Self::AgentHistory => 2,
            Self::AiModels => 3,
            Self::Docker => 4,
            Self::SystemApps => 5,
            Self::Other => 6,
        }
    }
}

/// Review posture of one row. This is not an authorization to delete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewBand {
    /// Selectable and conservatively recommended by the scanner.
    Recommended,
    /// Selectable, but not recommended until a person reviews it.
    NeedsReview,
    /// Inspect-only, protected, or ignored. It stays visible and unselected.
    NotSelectable,
}

impl ReviewBand {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Recommended => "recommended",
            Self::NeedsReview => "review",
            Self::NotSelectable => "blocked",
        }
    }
}

/// One accepted row borrowed for a single projection.
#[derive(Clone, Copy, Debug)]
pub struct MapInput<'a> {
    pub id: &'a str,
    pub rule_id: &'a str,
    pub path: Option<&'a str>,
    pub bytes: u64,
    /// When false, `bytes` is only a lower bound.
    pub size_complete: bool,
    pub selected_by_default: bool,
    pub selectable: bool,
}

/// Counts over the rows passed to the projection. Bytes are estimates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapSummary {
    pub mapped_bytes: u64,
    pub mapped_count: usize,
    pub recommended_bytes: u64,
    pub recommended_count: usize,
    pub review_bytes: u64,
    pub review_count: usize,
    pub blocked_bytes: u64,
    pub blocked_count: usize,
    pub partial_count: usize,
}

/// One leaf inside a parent tile.
#[derive(Clone, Debug, PartialEq)]
pub struct MapLeaf {
    /// Target id. Absent for an aggregated remainder.
    pub id: Option<String>,
    pub title: String,
    pub bytes: u64,
    pub category: MapCategory,
    pub band: ReviewBand,
    pub represented: usize,
    pub other: bool,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// One parent tile. Coordinates are relative to the map canvas.
#[derive(Clone, Debug, PartialEq)]
pub struct MapGroup {
    pub title: String,
    pub bytes: u64,
    pub category: MapCategory,
    pub represented: usize,
    pub other: bool,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub leaves: Vec<MapLeaf>,
}

/// Bounded treemap plus the summary of every input row.
#[derive(Clone, Debug, PartialEq)]
pub struct DiskMap {
    pub summary: MapSummary,
    pub groups: Vec<MapGroup>,
    pub categories: Vec<MapCategory>,
}

#[derive(Clone, Copy, Debug)]
pub struct MapBounds {
    pub width: f32,
    pub height: f32,
}

/// Classify a rule id. Unknown ids stay [`MapCategory::Other`].
pub fn category_of(rule_id: &str) -> MapCategory {
    if rule_id == "docker" || rule_id.starts_with("docker.") {
        return MapCategory::Docker;
    }
    if rule_id.starts_with("huggingface.") {
        return MapCategory::AiModels;
    }
    if rule_id.starts_with("ai.") {
        return MapCategory::AgentHistory;
    }
    if rule_id == "cargo.home.inspect" || rule_id.starts_with("jetbrains.") {
        return MapCategory::SystemApps;
    }
    if is_build_output(rule_id) {
        return MapCategory::BuildOutput;
    }
    if is_safe_to_clear(rule_id) {
        return MapCategory::SafeToClear;
    }
    MapCategory::Other
}

pub fn review_band(selectable: bool, selected_by_default: bool) -> ReviewBand {
    if !selectable {
        ReviewBand::NotSelectable
    } else if selected_by_default {
        ReviewBand::Recommended
    } else {
        ReviewBand::NeedsReview
    }
}

/// Short tile name. A path's last component wins over the rule id.
pub fn leaf_title(rule_id: &str, path: Option<&str>) -> String {
    if let Some(name) = path.and_then(path_name) {
        if !name.is_empty() {
            return name.to_string();
        }
    }
    rule_short_name(rule_id).to_string()
}

pub fn summarize(rows: &[MapInput<'_>]) -> MapSummary {
    let mut summary = MapSummary {
        mapped_bytes: 0,
        mapped_count: rows.len(),
        recommended_bytes: 0,
        recommended_count: 0,
        review_bytes: 0,
        review_count: 0,
        blocked_bytes: 0,
        blocked_count: 0,
        partial_count: 0,
    };
    for row in rows {
        summary.mapped_bytes = summary.mapped_bytes.saturating_add(row.bytes);
        if !row.size_complete {
            summary.partial_count += 1;
        }
        match review_band(row.selectable, row.selected_by_default) {
            ReviewBand::Recommended => {
                summary.recommended_bytes = summary.recommended_bytes.saturating_add(row.bytes);
                summary.recommended_count += 1;
            }
            ReviewBand::NeedsReview => {
                summary.review_bytes = summary.review_bytes.saturating_add(row.bytes);
                summary.review_count += 1;
            }
            ReviewBand::NotSelectable => {
                summary.blocked_bytes = summary.blocked_bytes.saturating_add(row.bytes);
                summary.blocked_count += 1;
            }
        }
    }
    summary
}

/// Project rows into a bounded squarified map. Zero-byte rows stay in the
/// summary and do not receive a tile. The function does not allocate a tile
/// per input row.
pub fn project_disk_map(rows: &[MapInput<'_>], bounds: MapBounds) -> DiskMap {
    let summary = summarize(rows);
    let width = finite_positive(bounds.width);
    let height = finite_positive(bounds.height);
    let mut indexed: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.bytes > 0)
        .map(|(index, _)| index)
        .collect();
    indexed.sort_by(|&left, &right| {
        rows[right]
            .bytes
            .cmp(&rows[left].bytes)
            .then_with(|| rows[left].id.cmp(rows[right].id))
    });
    let detail_end = indexed.len().min(MAX_DETAILED_ROWS);
    let tail = &indexed[detail_end..];
    let mut tail_bytes = 0u64;
    let mut tail_count = 0usize;
    for index in tail {
        tail_bytes = tail_bytes.saturating_add(rows[*index].bytes);
        tail_count += 1;
    }

    let mut grouped: HashMap<String, RawGroup> = HashMap::new();
    for index in &indexed[..detail_end] {
        let row = &rows[*index];
        let (key, title) = group_identity(row.rule_id, row.path);
        let group = grouped.entry(key).or_insert_with(|| RawGroup {
            title,
            bytes: 0,
            rows: Vec::new(),
        });
        group.bytes = group.bytes.saturating_add(row.bytes);
        group.rows.push(*index);
    }
    let mut groups: Vec<RawGroup> = grouped.into_values().collect();
    groups.sort_by(|left, right| {
        right
            .bytes
            .cmp(&left.bytes)
            .then_with(|| left.title.cmp(&right.title))
    });

    let needs_other = groups.len() > MAX_GROUPS || tail_count > 0;
    let keep = if needs_other {
        MAX_GROUPS.saturating_sub(1)
    } else {
        MAX_GROUPS
    };
    let mut other_bytes = tail_bytes;
    let mut other_count = tail_count;
    if groups.len() > keep {
        for group in groups.drain(keep..) {
            other_bytes = other_bytes.saturating_add(group.bytes);
            other_count += group.rows.len();
        }
    }

    let mut laid_groups = Vec::new();
    for group in &groups {
        laid_groups.push(finish_group(group, rows, false));
    }
    if other_count > 0 && other_bytes > 0 {
        laid_groups.push(MapGroup {
            title: String::new(),
            bytes: other_bytes,
            category: MapCategory::Other,
            represented: other_count,
            other: true,
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            leaves: vec![MapLeaf {
                id: None,
                title: String::new(),
                bytes: other_bytes,
                category: MapCategory::Other,
                band: ReviewBand::NeedsReview,
                represented: other_count,
                other: true,
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            }],
        });
    }

    let total_bytes = laid_groups
        .iter()
        .fold(0u64, |total, group| total.saturating_add(group.bytes));
    if total_bytes > 0 && width > 0.0 && height > 0.0 {
        let scale = (width * height) / total_bytes as f64;
        let data = laid_groups
            .iter()
            .enumerate()
            .map(|(index, group)| Datum {
                index,
                key: group_sort_key(group),
                area: group.bytes as f64 * scale,
            })
            .collect();
        for (index, rect) in squarify(data, Rect::new(0.0, 0.0, width, height)) {
            let group = &mut laid_groups[index];
            group.x = rect.x as f32;
            group.y = rect.y as f32;
            group.width = rect.width as f32;
            group.height = rect.height as f32;
            layout_leaves(group);
        }
    }

    let mut categories = Vec::new();
    for group in &laid_groups {
        push_category(&mut categories, group.category);
        for leaf in &group.leaves {
            push_category(&mut categories, leaf.category);
        }
    }
    categories.sort_by_key(|category| category.rank());

    DiskMap {
        summary,
        groups: laid_groups,
        categories,
    }
}

/// Fill used for a category tile. High contrast ignores the pastel.
pub fn category_fill(category: MapCategory, high_contrast: bool) -> u32 {
    if high_contrast {
        return 0x000000;
    }
    match category {
        MapCategory::BuildOutput => 0xD6E4F5,
        MapCategory::SafeToClear => 0xD7F0DE,
        MapCategory::AgentHistory => 0xE4D9FB,
        MapCategory::AiModels => 0xF8D5DE,
        MapCategory::Docker => 0xD4F1F4,
        MapCategory::SystemApps => 0xE6E4DF,
        MapCategory::Other => 0xF3E6D4,
    }
}

pub fn category_border(category: MapCategory, high_contrast: bool) -> u32 {
    if high_contrast {
        return 0xFFFFFF;
    }
    match category {
        MapCategory::BuildOutput => 0x8FB4D6,
        MapCategory::SafeToClear => 0x8FCB9E,
        MapCategory::AgentHistory => 0xB7A3E0,
        MapCategory::AiModels => 0xE3A8B6,
        MapCategory::Docker => 0x8FCDD4,
        MapCategory::SystemApps => 0xC5C1B8,
        MapCategory::Other => 0xE0CDB4,
    }
}

pub fn category_ink(high_contrast: bool) -> u32 {
    if high_contrast { 0xFFFFFF } else { 0x1C2430 }
}

fn is_build_output(rule_id: &str) -> bool {
    matches!(
        rule_id,
        "rust.target"
            | "rust.target.shared"
            | "rust.target.metadata_fallback_trash"
            | "node.node_modules"
            | "node.dist"
            | "node.build"
            | "node.coverage"
            | "node.nuxt_output"
            | "maven.target"
            | "gradle.build"
            | "dotnet.bin"
            | "dotnet.obj"
            | "python.venv"
            | "python.venv_dot"
    )
}

fn is_safe_to_clear(rule_id: &str) -> bool {
    matches!(
        rule_id,
        "node.next_cache"
            | "node.turbo"
            | "node.parcel_cache"
            | "node.nuxt"
            | "node.svelte_kit"
            | "node.angular_cache"
            | "node.astro"
            | "python.__pycache__"
            | "python.pytest_cache"
            | "python.mypy_cache"
            | "python.ruff_cache"
            | "python.nox"
            | "python.tox"
            | "npm.cache.clean"
            | "pnpm.store.prune"
            | "yarn.cache.clean"
            | "yarn.cache.clean.classic"
            | "yarn.cache.clean.modern"
            | "pip.cache.purge"
            | "uv.cache.clean"
            | "bun.cache.clean"
            | "go.mod_cache.clean"
            | "go.mod_cache"
            | "go.build_cache.clean"
            | "gradle.caches"
            | "gradle.wrapper_dists"
            | "gradle.project_cache"
            | "maven.repository"
            | "nuget.packages"
            | "ivy.cache"
            | "windows.chrome.cache"
            | "windows.edge.cache"
            | "windows.firefox.cache"
            | "windows.vscode.cache"
            | "windows.cursor.cache"
            | "windows.user_temp.stale"
    )
}

fn rule_short_name(rule_id: &str) -> &str {
    match rule_id {
        "npm.cache.clean" => "npm cache",
        "pnpm.store.prune" => "pnpm store",
        "yarn.cache.clean" | "yarn.cache.clean.classic" | "yarn.cache.clean.modern" => "yarn cache",
        "pip.cache.purge" => "pip cache",
        "uv.cache.clean" => "uv cache",
        "bun.cache.clean" => "bun cache",
        "go.mod_cache.clean" | "go.mod_cache" => "go modules",
        "go.build_cache.clean" => "go build cache",
        "node.node_modules" => "node_modules",
        "node.next_cache" => ".next",
        "node.dist" => "dist",
        "node.build" => "build",
        "rust.target" | "rust.target.shared" | "rust.target.metadata_fallback_trash" => "target",
        "huggingface.hub" => "Hugging Face",
        "cargo.home.inspect" => "Cargo home",
        "docker" => "Docker",
        "python.pytest_cache" => ".pytest_cache",
        "python.mypy_cache" => ".mypy_cache",
        "python.__pycache__" => "__pycache__",
        "windows.chrome.cache" => "Chrome cache",
        "windows.edge.cache" => "Edge cache",
        "windows.firefox.cache" => "Firefox cache",
        "windows.vscode.cache" => "VS Code cache",
        "windows.cursor.cache" => "Cursor cache",
        "windows.user_temp.stale" => "user temp",
        other => other,
    }
}

fn path_name(path: &str) -> Option<&str> {
    let trimmed = path.trim_end_matches(['/', '\\']);
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed))
}

fn group_identity(rule_id: &str, path: Option<&str>) -> (String, String) {
    if let Some(path) = path
        && let Some((key, title)) = parent_identity(path)
    {
        return (key, title);
    }
    let provider = rule_id.split('.').next().unwrap_or(rule_id);
    (format!("global:{provider}"), provider.to_string())
}

fn parent_identity(path: &str) -> Option<(String, String)> {
    let trimmed = path.trim_end_matches(['/', '\\']);
    let index = trimmed.rfind(['/', '\\'])?;
    let parent = &trimmed[..index];
    if parent.is_empty() {
        return None;
    }
    let title = parent.rsplit(['/', '\\']).next().unwrap_or(parent);
    if title.is_empty() {
        return None;
    }
    Some((parent.to_string(), title.to_string()))
}

struct RawGroup {
    title: String,
    bytes: u64,
    rows: Vec<usize>,
}

fn finish_group(group: &RawGroup, rows: &[MapInput<'_>], other: bool) -> MapGroup {
    let mut leaves: Vec<MapLeaf> = group
        .rows
        .iter()
        .map(|index| {
            let row = &rows[*index];
            MapLeaf {
                id: Some(row.id.to_string()),
                title: leaf_title(row.rule_id, row.path),
                bytes: row.bytes,
                category: category_of(row.rule_id),
                band: review_band(row.selectable, row.selected_by_default),
                represented: 1,
                other: false,
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            }
        })
        .collect();
    leaves.sort_by(|left, right| {
        right.bytes.cmp(&left.bytes).then_with(|| {
            left.id
                .as_deref()
                .unwrap_or("")
                .cmp(right.id.as_deref().unwrap_or(""))
        })
    });
    if leaves.len() > MAX_LEAVES {
        let mut folded_bytes = 0u64;
        let mut folded = 0usize;
        for leaf in leaves.drain(MAX_LEAVES - 1..) {
            folded_bytes = folded_bytes.saturating_add(leaf.bytes);
            folded += leaf.represented;
        }
        leaves.push(MapLeaf {
            id: None,
            title: String::new(),
            bytes: folded_bytes,
            category: MapCategory::Other,
            band: ReviewBand::NeedsReview,
            represented: folded,
            other: true,
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        });
    }
    let category = dominant_category(&leaves);
    MapGroup {
        title: group.title.clone(),
        bytes: group.bytes,
        category,
        represented: group.rows.len(),
        other,
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
        leaves,
    }
}

fn dominant_category(leaves: &[MapLeaf]) -> MapCategory {
    let mut best = MapCategory::Other;
    let mut best_bytes = 0u64;
    let mut seen = HashMap::<MapCategory, u64>::new();
    for leaf in leaves {
        if leaf.other {
            continue;
        }
        let bytes = seen.entry(leaf.category).or_insert(0);
        *bytes = bytes.saturating_add(leaf.bytes);
    }
    for (category, bytes) in seen {
        if bytes > best_bytes || (bytes == best_bytes && category.rank() < best.rank()) {
            best = category;
            best_bytes = bytes;
        }
    }
    best
}

fn group_sort_key(group: &MapGroup) -> String {
    if group.other {
        "\u{10FFFF}".to_string()
    } else {
        group.title.clone()
    }
}

fn layout_leaves(group: &mut MapGroup) {
    let width = group.width as f64;
    let height = group.height as f64;
    if width <= 0.0 || height <= 0.0 || group.leaves.is_empty() {
        return;
    }
    let header = if height >= 36.0 && width >= 24.0 {
        GROUP_HEADER
    } else {
        0.0
    };
    let body_height = (height - header).max(0.0);
    if body_height <= 0.0 {
        return;
    }
    let total = group
        .leaves
        .iter()
        .fold(0u64, |total, leaf| total.saturating_add(leaf.bytes));
    if total == 0 {
        return;
    }
    let scale = (width * body_height) / total as f64;
    let data = group
        .leaves
        .iter()
        .enumerate()
        .map(|(index, leaf)| Datum {
            index,
            key: leaf.id.clone().unwrap_or_else(|| format!("other-{index}")),
            area: leaf.bytes as f64 * scale,
        })
        .collect();
    for (index, rect) in squarify(data, Rect::new(0.0, header, width, body_height)) {
        let leaf = &mut group.leaves[index];
        leaf.x = rect.x as f32;
        leaf.y = rect.y as f32;
        leaf.width = rect.width as f32;
        leaf.height = rect.height as f32;
    }
}

fn push_category(categories: &mut Vec<MapCategory>, category: MapCategory) {
    if !categories.contains(&category) {
        categories.push(category);
    }
}

fn finite_positive(value: f32) -> f64 {
    if value.is_finite() && value > 0.0 {
        f64::from(value)
    } else {
        1.0
    }
}

#[derive(Clone)]
struct Datum {
    index: usize,
    key: String,
    area: f64,
}

#[derive(Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Rect {
    fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

fn squarify(mut data: Vec<Datum>, bounds: Rect) -> Vec<(usize, Rect)> {
    data.retain(|item| item.area.is_finite() && item.area > 0.0);
    data.sort_by(|left, right| {
        right
            .area
            .total_cmp(&left.area)
            .then_with(|| left.key.cmp(&right.key))
    });
    let mut output = Vec::new();
    let mut remaining = bounds;
    let mut row = Vec::new();
    while !data.is_empty() {
        let next = data[0].clone();
        let side = remaining.width.min(remaining.height);
        let mut trial = row.clone();
        trial.push(next.clone());
        if row.is_empty() || worst(&trial, side) <= worst(&row, side) {
            row.push(next);
            data.remove(0);
        } else {
            remaining = layout_row(&row, remaining, &mut output);
            row.clear();
        }
    }
    if !row.is_empty() {
        layout_row(&row, remaining, &mut output);
    }
    output
}

fn worst(row: &[Datum], side: f64) -> f64 {
    if row.is_empty() || side <= 0.0 {
        return f64::INFINITY;
    }
    let sum: f64 = row.iter().map(|item| item.area).sum();
    let largest = row[0].area;
    let smallest = row[row.len() - 1].area;
    if sum <= 0.0 || smallest <= 0.0 {
        return f64::INFINITY;
    }
    let side_sq = side * side;
    let sum_sq = sum * sum;
    (side_sq * largest / sum_sq).max(sum_sq / (side_sq * smallest))
}

fn layout_row(row: &[Datum], remaining: Rect, output: &mut Vec<(usize, Rect)>) -> Rect {
    let row_area: f64 = row.iter().map(|item| item.area).sum();
    if remaining.width >= remaining.height {
        let row_height = if remaining.width <= 0.0 {
            0.0
        } else {
            row_area / remaining.width
        };
        let mut x = remaining.x;
        for (index, item) in row.iter().enumerate() {
            let width = if index + 1 == row.len() {
                remaining.x + remaining.width - x
            } else if row_height <= 0.0 {
                0.0
            } else {
                item.area / row_height
            };
            output.push((
                item.index,
                Rect::new(x, remaining.y, width.max(0.0), row_height.max(0.0)),
            ));
            x += width;
        }
        Rect::new(
            remaining.x,
            remaining.y + row_height,
            remaining.width,
            (remaining.height - row_height).max(0.0),
        )
    } else {
        let row_width = if remaining.height <= 0.0 {
            0.0
        } else {
            row_area / remaining.height
        };
        let mut y = remaining.y;
        for (index, item) in row.iter().enumerate() {
            let height = if index + 1 == row.len() {
                remaining.y + remaining.height - y
            } else if row_width <= 0.0 {
                0.0
            } else {
                item.area / row_width
            };
            output.push((
                item.index,
                Rect::new(remaining.x, y, row_width.max(0.0), height.max(0.0)),
            ));
            y += height;
        }
        Rect::new(
            remaining.x + row_width,
            remaining.y,
            (remaining.width - row_width).max(0.0),
            remaining.height,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input<'a>(
        id: &'a str,
        rule_id: &'a str,
        path: Option<&'a str>,
        bytes: u64,
        selected_by_default: bool,
        selectable: bool,
    ) -> MapInput<'a> {
        MapInput {
            id,
            rule_id,
            path,
            bytes,
            size_complete: true,
            selected_by_default,
            selectable,
        }
    }

    #[test]
    fn build_output_and_caches_keep_distinct_categories() {
        assert_eq!(category_of("node.node_modules"), MapCategory::BuildOutput);
        assert_eq!(category_of("rust.target"), MapCategory::BuildOutput);
        assert_eq!(category_of("node.dist"), MapCategory::BuildOutput);
        assert_eq!(category_of("dotnet.bin"), MapCategory::BuildOutput);
        assert_eq!(category_of("node.next_cache"), MapCategory::SafeToClear);
        assert_eq!(category_of("npm.cache.clean"), MapCategory::SafeToClear);
        assert_eq!(category_of("pnpm.store.prune"), MapCategory::SafeToClear);
        assert_eq!(category_of("python.pytest_cache"), MapCategory::SafeToClear);
        assert_eq!(category_of("huggingface.hub"), MapCategory::AiModels);
        assert_eq!(category_of("docker"), MapCategory::Docker);
        assert_eq!(category_of("cargo.home.inspect"), MapCategory::SystemApps);
        assert_eq!(category_of("ai.codex.sessions"), MapCategory::AgentHistory);
        assert_eq!(category_of("unknown.rule"), MapCategory::Other);
        assert_eq!(review_band(true, true), ReviewBand::Recommended);
        assert_eq!(review_band(true, false), ReviewBand::NeedsReview);
        assert_eq!(review_band(false, true), ReviewBand::NotSelectable);
    }

    #[test]
    fn same_project_nests_build_output_under_its_parent() {
        let map = project_disk_map(
            &[
                input(
                    "modules",
                    "node.node_modules",
                    Some("C:/work/app/node_modules"),
                    80,
                    false,
                    true,
                ),
                input(
                    "target",
                    "rust.target",
                    Some("C:/work/app/target"),
                    20,
                    true,
                    true,
                ),
                input(
                    "next",
                    "node.next_cache",
                    Some("D:/other/web/.next/cache"),
                    10,
                    true,
                    true,
                ),
            ],
            MapBounds {
                width: 200.0,
                height: 100.0,
            },
        );
        assert_eq!(map.summary.mapped_count, 3);
        assert_eq!(map.summary.mapped_bytes, 110);
        assert_eq!(map.summary.recommended_count, 2);
        assert_eq!(map.summary.review_count, 1);
        assert_eq!(map.groups.len(), 2);
        let app = map
            .groups
            .iter()
            .find(|group| group.title == "app")
            .expect("app group");
        assert_eq!(app.category, MapCategory::BuildOutput);
        assert_eq!(app.bytes, 100);
        assert_eq!(app.leaves.len(), 2);
        assert!(app.leaves.iter().any(|leaf| leaf.title == "node_modules"));
        assert!(app.leaves.iter().any(|leaf| leaf.title == "target"));
        assert!(app.width > 0.0 && app.height > 0.0);
        let next = map
            .groups
            .iter()
            .find(|group| group.title == ".next")
            .expect(".next group");
        assert_eq!(next.category, MapCategory::SafeToClear);
        assert!(
            next.leaves
                .iter()
                .any(|leaf| leaf.id.as_deref() == Some("next"))
        );
    }

    #[test]
    fn larger_group_gets_more_area_and_tiles_stay_inside() {
        let map = project_disk_map(
            &[
                input(
                    "big",
                    "rust.target",
                    Some("C:/work/big/target"),
                    75,
                    true,
                    true,
                ),
                input(
                    "small",
                    "node.dist",
                    Some("C:/work/small/dist"),
                    25,
                    true,
                    true,
                ),
            ],
            MapBounds {
                width: 100.0,
                height: 100.0,
            },
        );
        assert_eq!(map.groups.len(), 2);
        let big = map
            .groups
            .iter()
            .find(|group| group.title == "big")
            .unwrap();
        let small = map
            .groups
            .iter()
            .find(|group| group.title == "small")
            .unwrap();
        let big_area = f64::from(big.width) * f64::from(big.height);
        let small_area = f64::from(small.width) * f64::from(small.height);
        let ratio = big_area / small_area;
        assert!(
            (ratio - 3.0).abs() < 0.2,
            "area ratio {ratio} big={big_area} small={small_area}"
        );
        for group in &map.groups {
            assert!(group.x >= -0.05, "{group:?}");
            assert!(group.y >= -0.05, "{group:?}");
            assert!(group.x + group.width <= 100.5, "{group:?}");
            assert!(group.y + group.height <= 100.5, "{group:?}");
            for leaf in &group.leaves {
                assert!(leaf.x + leaf.width <= group.width + 0.5, "{leaf:?}");
                assert!(leaf.y + leaf.height <= group.height + 0.5, "{leaf:?}");
            }
        }
    }

    #[test]
    fn over_cap_folds_into_other_without_dropping_bytes() {
        let ids: Vec<String> = (0..20).map(|index| format!("id-{index}")).collect();
        let paths: Vec<String> = (0..20)
            .map(|index| format!("C:/proj-{index:02}/target"))
            .collect();
        let inputs: Vec<MapInput<'_>> = ids
            .iter()
            .zip(paths.iter())
            .enumerate()
            .map(|(index, (id, path))| {
                input(
                    id,
                    "rust.target",
                    Some(path),
                    100 * (20 - index as u64),
                    true,
                    true,
                )
            })
            .collect();
        let map = project_disk_map(
            &inputs,
            MapBounds {
                width: 300.0,
                height: 200.0,
            },
        );
        let painted: u64 = map.groups.iter().map(|group| group.bytes).sum();
        assert_eq!(painted, 21_000);
        assert!(map.groups.len() <= MAX_GROUPS);
        let other = map.groups.iter().find(|group| group.other).expect("other");
        assert!(other.represented >= 9);
        assert_eq!(
            map.groups
                .iter()
                .filter(|group| group.title == "proj-00")
                .count(),
            1
        );
        let again = project_disk_map(
            &inputs,
            MapBounds {
                width: 300.0,
                height: 200.0,
            },
        );
        assert_eq!(map.groups.len(), again.groups.len());
        for (left, right) in map.groups.iter().zip(again.groups.iter()) {
            assert_eq!(left.title, right.title);
            assert_eq!(left.bytes, right.bytes);
            assert!((left.x - right.x).abs() < 0.01);
            assert!((left.width - right.width).abs() < 0.01);
        }
    }

    #[test]
    fn zero_byte_rows_stay_in_the_summary_and_out_of_tiles() {
        let map = project_disk_map(
            &[
                input(
                    "empty",
                    "node.dist",
                    Some("C:/work/app/dist"),
                    0,
                    true,
                    true,
                ),
                input("blocked", "cargo.home.inspect", None, 0, false, false),
            ],
            MapBounds {
                width: 100.0,
                height: 80.0,
            },
        );
        assert_eq!(map.summary.mapped_count, 2);
        assert_eq!(map.summary.mapped_bytes, 0);
        assert_eq!(map.summary.blocked_count, 1);
        assert!(map.groups.is_empty());
        assert_eq!(leaf_title("npm.cache.clean", None), "npm cache");
        assert_eq!(
            leaf_title("node.node_modules", Some("C:/work/app/node_modules")),
            "node_modules"
        );
    }
}
