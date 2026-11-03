//! Shell copy for the five modes and the beaver-menu surfaces.
//!
//! Strings match the existing EN and zh-CN catalogue ids. No network fetch.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Locale {
    En,
    ZhCn,
}

pub fn menu_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "DevSweep menu",
        Locale::ZhCn => "DevSweep 菜单",
    }
}

pub fn modes_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Modes",
        Locale::ZhCn => "模式",
    }
}

pub fn mode_name(locale: Locale, name: &'static str) -> &'static str {
    match (locale, name) {
        (Locale::En, "clean") => "Clean",
        (Locale::En, "software") => "Software",
        (Locale::En, "optimize") => "Optimize",
        (Locale::En, "analyze") => "Analyze",
        (Locale::En, "status") => "Status",
        (Locale::ZhCn, "clean") => "清理",
        (Locale::ZhCn, "software") => "软件",
        (Locale::ZhCn, "optimize") => "优化",
        (Locale::ZhCn, "analyze") => "分析",
        (Locale::ZhCn, "status") => "状态",
        _ => name,
    }
}

pub fn mode_subtitle(locale: Locale, name: &'static str) -> &'static str {
    match (locale, name) {
        (Locale::En, "clean") => "Review every Cleanup Target before anything moves.",
        (Locale::En, "software") => "Review current-user software inventory before uninstall.",
        (Locale::En, "optimize") => "Review catalogued actions before any run.",
        (Locale::En, "analyze") => "Map disk use. Nothing moves unless you confirm it.",
        (Locale::En, "status") => "Host facts from collectors. No score.",
        (Locale::ZhCn, "clean") => "先审查每个清理目标，再移动任何内容。",
        (Locale::ZhCn, "software") => "审查当前用户软件清单后再卸载。",
        (Locale::ZhCn, "optimize") => "审查目录中的操作后再运行。",
        (Locale::ZhCn, "analyze") => "映射磁盘占用。未经确认不会移动任何文件。",
        (Locale::ZhCn, "status") => "来自采集器的主机事实。无评分。",
        _ => "",
    }
}

/// The mode chord. `left` and `right` alone stay with the Note's caret, so the
/// shell needs its own chord that the text field does not claim.
pub fn mode_keys_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Keyboard: Alt+Left and Alt+Right switch mode while the Note has focus. F10 opens the menu; Down moves the highlighted item; Enter opens it."
        }
        Locale::ZhCn => {
            "键盘：焦点在备注输入框时，Alt+← 和 Alt+→ 切换模式。F10 打开菜单；Down 移动高亮项；Enter 打开它。"
        }
    }
}

pub fn settings_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Settings",
        Locale::ZhCn => "设置",
    }
}

pub fn settings_subtitle(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Appearance, language, and sampling preferences.",
        Locale::ZhCn => "外观、语言和采样配置。",
    }
}

pub fn protection_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Protection",
        Locale::ZhCn => "保护",
    }
}

pub fn protection_subtitle(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Inspect protection policy without changing cleanup authority.",
        Locale::ZhCn => "查看保护策略，不改变清理权限。",
    }
}

pub fn rules_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Rules",
        Locale::ZhCn => "规则",
    }
}

pub fn rules_subtitle(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Inspect the rule catalogue.",
        Locale::ZhCn => "查看规则目录。",
    }
}

pub fn rules_empty(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No shipped rules are available.",
        Locale::ZhCn => "没有可用的内置规则。",
    }
}

pub fn history_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "History",
        Locale::ZhCn => "历史",
    }
}

pub fn history_subtitle(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Inspect past cleanup records.",
        Locale::ZhCn => "查看既往清理记录。",
    }
}

pub fn history_empty(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No audit operations in the fixed V1 stores.",
        Locale::ZhCn => "固定 V1 存储中没有审计操作。",
    }
}

pub fn history_unavailable(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "The audit history store could not be read.",
        Locale::ZhCn => "无法读取审计历史存储。",
    }
}

pub fn history_operation_line(domain: &str, operation_id: &str, outcome: &str) -> String {
    format!("{domain} | {operation_id} | {outcome}")
}

pub fn history_no_replay(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "History is inspect-only. DevSweep cannot execute or replay a listed operation."
        }
        Locale::ZhCn => "历史仅供检查。DevSweep 不能执行或重放列出的操作。",
    }
}

pub fn save_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Save",
        Locale::ZhCn => "保存",
    }
}

pub fn back_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Back",
        Locale::ZhCn => "返回",
    }
}

pub fn note_placeholder(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Type here",
        Locale::ZhCn => "在此输入",
    }
}

pub fn language_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Language",
        Locale::ZhCn => "语言",
    }
}

pub fn language_option(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "English",
        Locale::ZhCn => "简体中文",
    }
}

pub fn theme_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Theme",
        Locale::ZhCn => "主题",
    }
}

pub fn project_folders(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Project folders",
        Locale::ZhCn => "项目文件夹",
    }
}

pub fn separate_protection(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Removing a selection does not exclude folders still covered by a selected parent. Cleanup protection is managed separately."
        }
        Locale::ZhCn => "移除选择不会排除仍被所选上级目录覆盖的文件夹。清理保护需单独设置。",
    }
}

pub fn stale_scan_roots(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Scan folders changed. Rescan before selecting or cleaning targets.",
        Locale::ZhCn => "扫描文件夹已改变，请重新扫描后再选择或清理。",
    }
}

pub fn stale_protection(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Protection rules changed. Rescan before selecting or cleaning targets.",
        Locale::ZhCn => "保护规则已改变，请重新扫描后再选择或清理。",
    }
}

pub fn workbench_provider_ui_pending(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "The provider UI is still pending.",
        Locale::ZhCn => "提供程序界面仍在等待处理。",
    }
}

pub fn workbench_privilege_decline(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Elevation was declined.",
        Locale::ZhCn => "已拒绝提升权限。",
    }
}

pub fn workbench_reboot_required(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "A reboot is required.",
        Locale::ZhCn => "需要重新启动计算机。",
    }
}

pub fn workbench_failed_uninstall(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "The uninstall failed.",
        Locale::ZhCn => "卸载失败。",
    }
}

/// Plain-language name for an Analyze class. Unknown is the common case and
/// reads as "no special class", not as an error.
#[must_use]
pub fn analyze_class_label(locale: Locale, class: &str) -> &'static str {
    match (locale, class) {
        (Locale::En, "Hibernation") => "Hibernation file",
        (Locale::En, "PageFile") => "Page or swap file",
        (Locale::En, "WslDisk") => "WSL virtual disk",
        (Locale::En, "DockerDisk") => "Docker virtual disk",
        (Locale::En, "AiModel") => "AI model",
        (Locale::En, "Backup") => "Backup",
        (Locale::En, "Game") => "Game install",
        (Locale::En, "CloudPlaceholder") => "Cloud placeholder",
        (Locale::En, _) => "No special class",
        (Locale::ZhCn, "Hibernation") => "休眠文件",
        (Locale::ZhCn, "PageFile") => "分页或交换文件",
        (Locale::ZhCn, "WslDisk") => "WSL 虚拟磁盘",
        (Locale::ZhCn, "DockerDisk") => "Docker 虚拟磁盘",
        (Locale::ZhCn, "AiModel") => "AI 模型",
        (Locale::ZhCn, "Backup") => "备份",
        (Locale::ZhCn, "Game") => "游戏安装",
        (Locale::ZhCn, "CloudPlaceholder") => "云端占位文件",
        (Locale::ZhCn, _) => "无特殊类别",
    }
}

pub fn software_leftovers_unavailable(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Leftovers is not a software workbench segment. It is unavailable here.",
        Locale::ZhCn => "残留项不是软件工作台分段，这里不可用。",
    }
}

pub fn workbench_not_run(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "not run",
        Locale::ZhCn => "未运行",
    }
}

pub fn workbench_host_skip(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "not run: host volume and image are not disposable targets",
        Locale::ZhCn => "未运行：主机卷和映像不是可丢弃目标",
    }
}

pub fn workbench_privilege(locale: Locale, class_id: &str) -> &'static str {
    match (locale, class_id) {
        (Locale::En, "no_dispatch") => "no dispatch",
        (Locale::ZhCn, "no_dispatch") => "不可调度",
        (Locale::En, "no_elevation") => "no elevation",
        (Locale::ZhCn, "no_elevation") => "不需要提升",
        (Locale::En, "plan_time") => "elevation checked at plan time",
        (Locale::ZhCn, "plan_time") => "计划时检查提升",
        _ => "not run",
    }
}

pub fn analyze_face_label(locale: Locale, face: &str) -> &'static str {
    match (locale, face) {
        (Locale::En, "empty") => "empty",
        (Locale::ZhCn, "empty") => "空",
        (Locale::En, "unreadable") => "unreadable",
        (Locale::ZhCn, "unreadable") => "不可读",
        (Locale::En, "present") => "present",
        (Locale::ZhCn, "present") => "存在",
        (Locale::En, _) => "present",
        (Locale::ZhCn, _) => "存在",
    }
}

pub fn workbench_changed_selection(locale: Locale, selection_id: &str) -> String {
    match locale {
        Locale::En => format!(
            "The selection changed. The previous plan is not confirmed. Selection id: {selection_id}."
        ),
        Locale::ZhCn => format!("选择已更改。先前计划未确认。选择标识：{selection_id}。"),
    }
}

pub fn clean_outcome_unavailable(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "The source is unavailable.",
        Locale::ZhCn => "来源不可用。",
    }
}

pub fn clean_outcome_empty(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "There is nothing to clean.",
        Locale::ZhCn => "没有可清理的内容。",
    }
}

pub fn clean_outcome_partial(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "The result is partial.",
        Locale::ZhCn => "结果为部分完成。",
    }
}

pub fn clean_outcome_failed(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "The clean failed.",
        Locale::ZhCn => "清理失败。",
    }
}

pub fn clean_outcome_canceled(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "The clean was canceled.",
        Locale::ZhCn => "清理已取消。",
    }
}

pub fn clean_outcome_stale_preview(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "The preview digest is stale.",
        Locale::ZhCn => "预览摘要已过期。",
    }
}

pub fn clean_outcome_confirmation_required(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Confirmation is required.",
        Locale::ZhCn => "需要确认。",
    }
}

pub fn clean_outcome_mixed(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Some targets moved and some did not.",
        Locale::ZhCn => "部分目标已移出，部分目标未移出。",
    }
}

pub fn clean_outcome_trash_reported(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "The isolated trash move was reported.",
        Locale::ZhCn => "已报告隔离的移出。",
    }
}

pub fn clean_scan_primary(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Start a Clean scan",
        Locale::ZhCn => "开始清理扫描",
    }
}

pub fn clean_scope_prompt(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Choose a scan scope. No scan starts until you pick one.",
        Locale::ZhCn => "选择扫描范围。选择之前不会开始扫描。",
    }
}

pub fn clean_scope_project(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Project artifacts",
        Locale::ZhCn => "项目产物",
    }
}

pub fn clean_scope_global(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Global caches",
        Locale::ZhCn => "全局缓存",
    }
}

pub fn clean_global_confirm(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Global caches live in your real user profile. Only this confirmation starts that scan."
        }
        Locale::ZhCn => "全局缓存位于真实用户目录。只有确认后才会开始该扫描。",
    }
}

pub fn clean_global_scan_action(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Scan global caches",
        Locale::ZhCn => "扫描全局缓存",
    }
}

pub fn clean_root_prompt(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "No project folder is saved yet. Pick the folder to scan; DevSweep keeps it as the explicit scan root."
        }
        Locale::ZhCn => {
            "尚未保存项目文件夹。请选择要扫描的文件夹，DevSweep 会将其保存为显式扫描根。"
        }
    }
}

pub fn clean_root_choose(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Choose folder…",
        Locale::ZhCn => "选择文件夹…",
    }
}

pub fn clean_root_suggestions(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Suggested folders",
        Locale::ZhCn => "建议文件夹",
    }
}

pub fn clean_root_line(locale: Locale, root: &str) -> String {
    match locale {
        Locale::En => format!("Scan root: {root}"),
        Locale::ZhCn => format!("扫描根：{root}"),
    }
}

pub fn clean_root_session_only(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "The folder could not be remembered. It stays active for this session only.",
        Locale::ZhCn => "该文件夹无法保存，仅在本次会话中生效。",
    }
}

pub fn clean_phase_scanning(locale: Locale, found: usize) -> String {
    match locale {
        Locale::En => format!("Scanning… {found} targets found so far."),
        Locale::ZhCn => format!("正在扫描……已发现 {found} 个目标。"),
    }
}

pub fn clean_phase_found(locale: Locale, count: usize) -> String {
    match locale {
        Locale::En => format!("Scan finished. {count} targets found."),
        Locale::ZhCn => format!("扫描完成，发现 {count} 个目标。"),
    }
}

pub fn clean_phase_all_clean(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "The scan finished with no targets.",
        Locale::ZhCn => "扫描完成，未发现目标。",
    }
}

pub fn clean_phase_cleaning(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Moving the selected targets…",
        Locale::ZhCn => "正在移出所选目标……",
    }
}

pub fn clean_phase_completed(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "The clean finished.",
        Locale::ZhCn => "清理已结束。",
    }
}

pub fn clean_phase_failed(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "The scan did not finish.",
        Locale::ZhCn => "扫描未完成。",
    }
}

pub fn clean_retry_action(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Scan again",
        Locale::ZhCn => "重新扫描",
    }
}

pub fn clean_cancel_action(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Cancel",
        Locale::ZhCn => "取消",
    }
}

pub fn clean_search_placeholder(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Search rows",
        Locale::ZhCn => "搜索行",
    }
}

pub fn clean_sort_report_order(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Sort: report order",
        Locale::ZhCn => "排序：报告顺序",
    }
}

pub fn clean_sort_name(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Sort: name",
        Locale::ZhCn => "排序：名称",
    }
}

pub fn clean_sort_size(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Sort: size",
        Locale::ZhCn => "排序：大小",
    }
}

pub fn clean_select_recommended(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Select recommended",
        Locale::ZhCn => "选择推荐项",
    }
}

pub fn clean_clear_selection(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Clear selection",
        Locale::ZhCn => "清除选择",
    }
}

pub fn clean_selection_count(locale: Locale, selected: usize, total: usize) -> String {
    match locale {
        Locale::En => format!("{selected} of {total} targets selected."),
        Locale::ZhCn => format!("已选择 {total} 个目标中的 {selected} 个。"),
    }
}

/// Bounded review summary of the selection set. `sample` holds the leading
/// ids (the caller bounds the list); the rest of the selection is a count.
pub fn clean_selection_summary(locale: Locale, sample: &[&str], selected: usize) -> String {
    let list = sample.join(", ");
    let hidden = selected.saturating_sub(sample.len());
    match locale {
        Locale::En if hidden == 0 => format!("Selected: {list}."),
        Locale::En => format!("Selected: {list} and {hidden} more."),
        Locale::ZhCn if hidden == 0 => format!("已选择：{list}。"),
        Locale::ZhCn => format!("已选择：{list}，另有 {hidden} 个。"),
    }
}

pub fn clean_preview_action(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Preview digest",
        Locale::ZhCn => "预览摘要",
    }
}

pub fn clean_preview_digest_line(locale: Locale, digest: &str, count: usize) -> String {
    match locale {
        Locale::En => format!("Live preview digest: {digest} ({count} selected)."),
        Locale::ZhCn => format!("实时预览摘要：{digest}（已选 {count} 个）。"),
    }
}

pub fn clean_confirm_action(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Confirm move",
        Locale::ZhCn => "确认移出",
    }
}

pub fn clean_confirm_required_line(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "A preview produced from the current selection is required. A stale digest never moves files."
        }
        Locale::ZhCn => "必须先生成与当前选择一致的预览。过期摘要不会移出任何文件。",
    }
}

pub fn clean_result_counts(locale: Locale, moved: usize, total: usize) -> String {
    match locale {
        Locale::En => format!("Moved {moved} of {total} selected targets."),
        Locale::ZhCn => format!("已移出所选 {total} 个目标中的 {moved} 个。"),
    }
}

pub fn clean_row_selected_marker(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "[x]",
        Locale::ZhCn => "[x]",
    }
}

pub fn clean_row_unselected_marker(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "[ ]",
        Locale::ZhCn => "[ ]",
    }
}

pub fn clean_row_global(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "global",
        Locale::ZhCn => "全局",
    }
}

pub fn clean_row_confidence_verified(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "verified size",
        Locale::ZhCn => "已验证大小",
    }
}

pub fn clean_row_confidence_partial(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "lower-bound size",
        Locale::ZhCn => "下限大小",
    }
}

pub fn clean_row_confidence_unknown(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "unknown size",
        Locale::ZhCn => "未知大小",
    }
}

pub fn clean_row_inspect_only(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "inspect-only",
        Locale::ZhCn => "仅检查",
    }
}

pub fn clean_row_protected(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "protected",
        Locale::ZhCn => "受保护",
    }
}

pub fn clean_row_ignored(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "ignored",
        Locale::ZhCn => "已忽略",
    }
}

pub fn clean_protect_action(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Protect row",
        Locale::ZhCn => "保护此行",
    }
}

pub fn clean_unprotect_action(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Remove protection",
        Locale::ZhCn => "移除保护",
    }
}

pub fn clean_ignore_action(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Ignore row",
        Locale::ZhCn => "忽略此行",
    }
}

pub fn clean_unignore_action(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Stop ignoring",
        Locale::ZhCn => "取消忽略",
    }
}

pub fn clean_protection_session_only(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "The protection list could not be saved. Protection stays active for this session only."
        }
        Locale::ZhCn => "保护列表无法保存，仅在本次会话中生效。",
    }
}

/// Why the cache will regrow and what regenerates it. One bounded sentence per
/// rule; unknown rule ids get an honest generic sentence, never a claim about
/// work the tool did not observe.
pub fn clean_row_rebuild_cost(locale: Locale, rule_id: &str) -> String {
    let text = match locale {
        Locale::En => match rule_id {
            "python.pytest_cache" => "pytest recreates it on the next test run.",
            "python.mypy_cache" => "mypy re-analyses and recreates it on the next type-check.",
            "python.ruff_cache" => "ruff recreates it on the next lint run.",
            "python.tox" => "tox recreates its environment on the next tox run.",
            "python.nox" => "nox recreates its virtualenv on the next nox run.",
            "python.venv_dot" | "python.venv" => {
                "pip or uv recreate the environment from the lock file on the next install."
            }
            "node.node_modules" => {
                "the package manager reinstalls it from the lockfile on the next install."
            }
            "node.dist" => "the bundler rebuilds the output on the next production build.",
            "node.build" => "the build tool recreates it on the next build.",
            "node.coverage" => "the next test run recreates the coverage output.",
            "node.next_cache" => "Next.js recreates .next on the next dev or build run.",
            "node.turbo" => "Turborepo repopulates it on the next pipeline run.",
            "node.parcel_cache" => "Parcel repopulates it on the next build.",
            "node.nuxt" => "Nuxt regenerates .nuxt on the next dev or build run.",
            "node.nuxt_output" => "Nuxt regenerates .output on the next build.",
            "node.svelte_kit" => "SvelteKit regenerates .svelte-kit on the next dev or build run.",
            "node.angular_cache" => "Angular regenerates .angular on the next CLI run.",
            "node.astro" => "Astro regenerates .astro on the next dev or build run.",
            "rust.target" | "rust.target.shared" => {
                "cargo rebuilds it on the next compile; incremental state is lost."
            }
            "maven.target" => "Maven rebuilds target/ on the next build.",
            "maven.repository" => "Maven re-downloads artifacts on the next build.",
            "gradle.build" | "gradle.project_cache" | "gradle.caches" | "gradle.wrapper_dists" => {
                "Gradle regenerates it on the next build or sync."
            }
            "dotnet.bin" => "dotnet build recreates bin/ on the next build.",
            "dotnet.obj" => "dotnet build recreates obj/ on the next build.",
            "go.mod_cache" => "go re-downloads modules on the next build.",
            "go.build_cache.clean" => "go rebuilds its compile cache on the next build.",
            "npm.cache.clean" => "npm repopulates its download cache on the next install.",
            "pip.cache.purge" => "pip re-downloads wheels on the next install.",
            "pnpm.store.prune" => "pnpm re-fetches packages on the next install.",
            "yarn.cache.clean" | "yarn.cache.clean.classic" | "yarn.cache.clean.modern" => {
                "Yarn repopulates its cache on the next install."
            }
            "uv.cache.clean" => "uv re-downloads packages on the next sync.",
            "bun.cache.clean" => "Bun repopulates its cache on the next install.",
            "windows.chrome.cache" => "Chrome rebuilds this cache while you browse.",
            "windows.edge.cache" => "Edge rebuilds this cache while you browse.",
            "windows.firefox.cache" => "Firefox rebuilds this cache while you browse.",
            "windows.cursor.cache" => "Cursor rebuilds this cache as you edit.",
            "windows.vscode.cache" => "VS Code rebuilds this cache as you edit.",
            "windows.shader.cache" => {
                "the GPU driver rebuilds the shader cache during play or render."
            }
            "windows.update.download" => "Windows Update re-downloads pending updates.",
            "windows.user_temp.stale" => "applications recreate temp files as they run.",
            "windows.wer.reports" => {
                "Windows Error Reporting writes new reports on the next crash."
            }
            "windows.delivery_optimization" => "downloads are re-fetched when updates run.",
            "huggingface.hub" => "models are re-downloaded from the Hub on the next use.",
            "jetbrains.caches" => "the IDE rebuilds its index on the next project open.",
            "ivy.cache" => "Ivy re-downloads dependencies on the next resolve.",
            "nuget.packages" => "NuGet re-downloads packages on the next restore.",
            "cargo.home.inspect" => "the Rust toolchain re-downloads crates on the next build.",
            "docker" => "images and layers are re-pulled the next time they are needed.",
            _ => "the owning tool recreates it the next time it runs.",
        },
        Locale::ZhCn => match rule_id {
            "python.pytest_cache" => "pytest 会在下次测试运行时重新生成。",
            "python.mypy_cache" => "mypy 会在下次类型检查时重新分析并生成。",
            "python.ruff_cache" => "ruff 会在下次检查运行中重新生成。",
            "python.tox" => "tox 会在下次运行中重建其环境。",
            "python.nox" => "nox 会在下次运行中重建其虚拟环境。",
            "python.venv_dot" | "python.venv" => "pip 或 uv 会在下次安装时按锁文件重建该环境。",
            "node.node_modules" => "包管理器会在下次安装时按锁文件重新安装。",
            "node.dist" => "打包器会在下次生产构建时重新构建输出。",
            "node.build" => "构建工具会在下次构建时重新生成。",
            "node.coverage" => "下次测试运行会重新生成覆盖率输出。",
            "node.next_cache" => "Next.js 会在下次开发或构建时重新生成 .next。",
            "node.turbo" => "Turborepo 会在下次流水线运行时重新填充。",
            "node.parcel_cache" => "Parcel 会在下次构建时重新填充。",
            "node.nuxt" => "Nuxt 会在下次开发或构建时重新生成 .nuxt。",
            "node.nuxt_output" => "Nuxt 会在下次构建时重新生成 .output。",
            "node.svelte_kit" => "SvelteKit 会在下次开发或构建时重新生成 .svelte-kit。",
            "node.angular_cache" => "Angular 会在下次 CLI 运行时重新生成 .angular。",
            "node.astro" => "Astro 会在下次开发或构建时重新生成 .astro。",
            "rust.target" | "rust.target.shared" => "cargo 会在下次编译时重建；增量状态会丢失。",
            "maven.target" => "Maven 会在下次构建时重建 target/。",
            "maven.repository" => "Maven 会在下次构建时重新下载依赖。",
            "gradle.build" | "gradle.project_cache" | "gradle.caches" | "gradle.wrapper_dists" => {
                "Gradle 会在下次构建或同步时重新生成。"
            }
            "dotnet.bin" => "dotnet build 会在下次构建时重新生成 bin/。",
            "dotnet.obj" => "dotnet build 会在下次构建时重新生成 obj/。",
            "go.mod_cache" => "go 会在下次构建时重新下载模块。",
            "go.build_cache.clean" => "go 会在下次构建时重建编译缓存。",
            "npm.cache.clean" => "npm 会在下次安装时重新填充下载缓存。",
            "pip.cache.purge" => "pip 会在下次安装时重新下载 wheel。",
            "pnpm.store.prune" => "pnpm 会在下次安装时重新获取包。",
            "yarn.cache.clean" | "yarn.cache.clean.classic" | "yarn.cache.clean.modern" => {
                "Yarn 会在下次安装时重新填充缓存。"
            }
            "uv.cache.clean" => "uv 会在下次同步时重新下载包。",
            "bun.cache.clean" => "Bun 会在下次安装时重新填充缓存。",
            "windows.chrome.cache" => "Chrome 会在浏览过程中重建该缓存。",
            "windows.edge.cache" => "Edge 会在浏览过程中重建该缓存。",
            "windows.firefox.cache" => "Firefox 会在浏览过程中重建该缓存。",
            "windows.cursor.cache" => "Cursor 会在编辑过程中重建该缓存。",
            "windows.vscode.cache" => "VS Code 会在编辑过程中重建该缓存。",
            "windows.shader.cache" => "GPU 驱动会在游戏或渲染时重建着色器缓存。",
            "windows.update.download" => "Windows 更新会在下次更新时重新下载。",
            "windows.user_temp.stale" => "应用程序会在运行时重新创建临时文件。",
            "windows.wer.reports" => "Windows 错误报告会在下次崩溃时写入新报告。",
            "windows.delivery_optimization" => "更新运行时会重新获取下载内容。",
            "huggingface.hub" => "模型会在下次使用时从 Hub 重新下载。",
            "jetbrains.caches" => "IDE 会在下次打开项目时重建索引。",
            "ivy.cache" => "Ivy 会在下次解析时重新下载依赖。",
            "nuget.packages" => "NuGet 会在下次还原时重新下载包。",
            "cargo.home.inspect" => "Rust 工具链会在下次构建时重新下载 crate。",
            "docker" => "镜像与层会在下次需要时重新拉取。",
            _ => "所属工具会在下次运行时重新生成。",
        },
    };
    match locale {
        Locale::En => format!("rebuild: {text}"),
        Locale::ZhCn => format!("重建：{text}"),
    }
}

pub fn clean_ai_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "AI tool review",
        Locale::ZhCn => "AI 工具审查",
    }
}

pub fn clean_ai_subtitle(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Review AI-tool caches, versions, sessions, and worktrees. Sensitive paths are never selectable."
        }
        Locale::ZhCn => "审查 AI 工具的缓存、版本、会话与工作树。敏感路径永远不可选择。",
    }
}

pub fn clean_ai_open_action(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "AI tools",
        Locale::ZhCn => "AI 工具",
    }
}

pub fn clean_ai_close_action(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Close AI review",
        Locale::ZhCn => "关闭 AI 审查",
    }
}

pub fn clean_ai_provider(locale: Locale, provider: &str) -> String {
    match locale {
        Locale::En => format!("Provider: {provider}"),
        Locale::ZhCn => format!("提供程序：{provider}"),
    }
}

pub fn clean_ai_scan_action(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Scan AI home",
        Locale::ZhCn => "扫描 AI 主目录",
    }
}

pub fn clean_ai_landing_prompt(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Nothing is scanned until you ask. The scan reads one provider home; every sensitive path stays unselected."
        }
        Locale::ZhCn => {
            "在你发起之前不会扫描。扫描只读取一个提供程序主目录；所有敏感路径保持未选中。"
        }
    }
}

pub fn clean_ai_phase_scanning(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Scanning the AI home…",
        Locale::ZhCn => "正在扫描 AI 主目录……",
    }
}

pub fn clean_ai_phase_review(locale: Locale, rows: usize, sensitive: usize) -> String {
    match locale {
        Locale::En => format!("AI review: {rows} rows observed, {sensitive} sensitive."),
        Locale::ZhCn => format!("AI 审查：观察到 {rows} 行，其中敏感 {sensitive} 行。"),
    }
}

pub fn clean_ai_phase_cleaning(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Moving the selected AI targets…",
        Locale::ZhCn => "正在移出所选 AI 目标……",
    }
}

pub fn clean_ai_phase_completed(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "The AI review action finished.",
        Locale::ZhCn => "AI 审查操作已结束。",
    }
}

pub fn clean_ai_phase_failed(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "The AI review action did not finish.",
        Locale::ZhCn => "AI 审查操作未完成。",
    }
}

pub fn clean_ai_category_name(locale: Locale, id: &str) -> &'static str {
    match (locale, id) {
        (Locale::En, "cache") => "cache",
        (Locale::En, "versions") => "versions",
        (Locale::En, "sessions") => "sessions",
        (Locale::En, "worktrees") => "worktrees",
        (Locale::En, "sensitive") => "sensitive",
        (Locale::ZhCn, "cache") => "缓存",
        (Locale::ZhCn, "versions") => "版本",
        (Locale::ZhCn, "sessions") => "会话",
        (Locale::ZhCn, "worktrees") => "工作树",
        (Locale::ZhCn, "sensitive") => "敏感",
        _ => "",
    }
}

/// Bounded category summary. `counts` pairs a category id with its row count.
pub fn clean_ai_category_counts(locale: Locale, counts: &[(&str, usize)]) -> String {
    let list = counts
        .iter()
        .map(|(id, count)| format!("{} {count}", clean_ai_category_name(locale, id)))
        .collect::<Vec<_>>()
        .join(" · ");
    match locale {
        Locale::En => format!("AI categories: {list}."),
        Locale::ZhCn => format!("AI 分类：{list}。"),
    }
}

pub fn clean_ai_active_category(locale: Locale, name: &str, selectable: usize) -> String {
    match locale {
        Locale::En => format!("Active AI category: {name} ({selectable} selectable)."),
        Locale::ZhCn => format!("当前 AI 分类：{name}（可选 {selectable} 项）。"),
    }
}

pub fn clean_ai_tool_rows(locale: Locale, count: usize) -> String {
    match locale {
        Locale::En => format!("{count} observed tool rows are not cleanup targets."),
        Locale::ZhCn => format!("观察到 {count} 个工具行，它们不是清理目标。"),
    }
}

pub fn clean_ai_selection_count(locale: Locale, selected: usize, selectable: usize) -> String {
    match locale {
        Locale::En => format!("AI selection: {selected} of {selectable} selectable rows."),
        Locale::ZhCn => format!("AI 选择：可选 {selectable} 行中的 {selected} 行。"),
    }
}

/// Selections kept in non-active AI categories. `list` is preformatted.
pub fn clean_ai_other_selections(locale: Locale, list: &str) -> String {
    match locale {
        Locale::En => format!("Selections in other AI categories: {list}."),
        Locale::ZhCn => format!("其他 AI 分类中的选择：{list}。"),
    }
}

pub fn clean_ai_digest_line(locale: Locale, digest: &str, count: usize) -> String {
    match locale {
        Locale::En => format!("AI preview digest: {digest} ({count} targets)."),
        Locale::ZhCn => format!("AI 预览摘要：{digest}（{count} 个目标）。"),
    }
}

pub fn clean_ai_confirm_action(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Confirm AI move",
        Locale::ZhCn => "确认 AI 移出",
    }
}

pub fn clean_ai_row_class(locale: Locale, class: &str) -> &'static str {
    match (locale, class) {
        (Locale::En, "regenerable_cache") => "cache",
        (Locale::En, "superseded_version") => "version",
        (Locale::En, "session_archive") => "session",
        (Locale::En, "worktree") => "worktree",
        (Locale::En, "installed_tool") => "tool",
        (Locale::ZhCn, "regenerable_cache") => "缓存",
        (Locale::ZhCn, "superseded_version") => "版本",
        (Locale::ZhCn, "session_archive") => "会话",
        (Locale::ZhCn, "worktree") => "工作树",
        (Locale::ZhCn, "installed_tool") => "工具",
        _ => "",
    }
}

pub fn clean_ai_row_selectable(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "selectable",
        Locale::ZhCn => "可选择",
    }
}

pub fn clean_ai_row_inspect_only(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "inspect-only",
        Locale::ZhCn => "仅检查",
    }
}

pub fn clean_ai_row_protected(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "protected",
        Locale::ZhCn => "受保护",
    }
}

pub fn clean_ai_row_sensitive(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "sensitive · never cleaned",
        Locale::ZhCn => "敏感 · 永不清理",
    }
}

pub fn clean_ai_row_unsupported(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "unsupported",
        Locale::ZhCn => "不支持",
    }
}

/// Locale-neutral core reason code as user copy. Unknown codes stay verbatim.
pub fn clean_ai_reason(locale: Locale, reason: &str) -> String {
    let mapped = match (locale, reason) {
        (Locale::En, "not_regen") => "not a regenerable cache",
        (Locale::En, "current_version") => "current version",
        (Locale::En, "current_unknown") => "current version unknown",
        (Locale::En, "active_session") => "active session",
        (Locale::En, "locked") => "locked",
        (Locale::En, "unknown_active_owner") => "active owner unknown",
        (Locale::En, "dirty") => "uncommitted changes",
        (Locale::En, "untracked") => "untracked files",
        (Locale::En, "ignored") => "ignored files",
        (Locale::En, "unpushed") => "unpushed commits",
        (Locale::En, "submodule") => "submodule state",
        (Locale::En, "shared_git_dir") => "shared git directory",
        (Locale::En, "active_owner") => "active owner",
        (Locale::En, "git_unavailable") => "git is unavailable",
        (Locale::En, "not_registered") => "not a registered worktree",
        (Locale::En, "unreadable") => "unreadable",
        (Locale::En, "bounded") => "exceeds the hash bound",
        (Locale::ZhCn, "not_regen") => "不是可再生成的缓存",
        (Locale::ZhCn, "current_version") => "当前版本",
        (Locale::ZhCn, "current_unknown") => "当前版本未知",
        (Locale::ZhCn, "active_session") => "活动会话",
        (Locale::ZhCn, "locked") => "已锁定",
        (Locale::ZhCn, "unknown_active_owner") => "活动归属未知",
        (Locale::ZhCn, "dirty") => "有未提交更改",
        (Locale::ZhCn, "untracked") => "有未跟踪文件",
        (Locale::ZhCn, "ignored") => "有被忽略文件",
        (Locale::ZhCn, "unpushed") => "有未推送提交",
        (Locale::ZhCn, "submodule") => "包含子模块",
        (Locale::ZhCn, "shared_git_dir") => "共享 git 目录",
        (Locale::ZhCn, "active_owner") => "活动归属",
        (Locale::ZhCn, "git_unavailable") => "git 不可用",
        (Locale::ZhCn, "not_registered") => "不是已注册的工作树",
        (Locale::ZhCn, "unreadable") => "不可读",
        (Locale::ZhCn, "bounded") => "超出哈希上限",
        _ => return reason.to_string(),
    };
    mapped.to_string()
}

pub fn clean_ai_recovery_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Recovery bundles",
        Locale::ZhCn => "恢复包",
    }
}

pub fn clean_ai_recovery_refresh(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Refresh recovery list",
        Locale::ZhCn => "刷新恢复列表",
    }
}

pub fn clean_ai_recovery_empty(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No recovery bundles under this home.",
        Locale::ZhCn => "此主目录下没有恢复包。",
    }
}

pub fn clean_ai_recovery_line(locale: Locale, id: &str, hash: &str, relative: &str) -> String {
    match locale {
        Locale::En => format!("Bundle {id} · content sha256 {hash} · {relative}."),
        Locale::ZhCn => format!("恢复包 {id} · 内容 sha256 {hash} · {relative}。"),
    }
}

pub fn clean_ai_restore_action(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Restore plan",
        Locale::ZhCn => "恢复计划",
    }
}

/// A built restore plan never copies payload bytes by itself.
pub fn clean_ai_restore_ready(locale: Locale, relative: &str, hash: &str) -> String {
    match locale {
        Locale::En => format!(
            "Restore plan ready for {relative} ({hash}). Building the plan does not copy bytes."
        ),
        Locale::ZhCn => {
            format!("已为 {relative} 生成恢复计划（{hash}）。生成计划不会复制任何字节。")
        }
    }
}

pub fn clean_ai_restore_refused(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "The restore plan was refused.",
        Locale::ZhCn => "恢复计划被拒绝。",
    }
}

pub fn clean_ai_home_unavailable(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No AI provider home is available from USERPROFILE/HOME.",
        Locale::ZhCn => "无法从 USERPROFILE/HOME 解析 AI 提供程序主目录。",
    }
}

/// Shown when the window has no explicit temporary AI home. It is a refusal,
/// not a scan of a real profile.
pub fn clean_ai_isolated_home_required(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "AI review needs an explicit temporary home. Real .codex, .claude, and Cursor profiles are not read."
        }
        Locale::ZhCn => "AI 审阅需要显式临时主目录。不会读取真实的 .codex、.claude 或 Cursor。",
    }
}

/// The absolute provider home the last explicit scan used. Not a claim that
/// the real user profile was read.
pub fn clean_ai_home_line(locale: Locale, home: &str) -> String {
    match locale {
        Locale::En => format!("AI home: {home}"),
        Locale::ZhCn => format!("AI 主目录：{home}"),
    }
}

pub fn clean_ai_keys_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Keyboard: F8 opens AI review without scanning; Enter scans or confirms the active AI category; Space toggles the focused row; Alt+Up/Alt+Down move the row cursor; Escape closes the AI review."
        }
        Locale::ZhCn => {
            "键盘：F8 打开 AI 审查且不扫描；Enter 扫描或确认当前 AI 分类；空格切换当前行；Alt+↑/Alt+↓ 移动行光标；Escape 关闭 AI 审查。"
        }
    }
}

pub fn history_clean_ai_executed(locale: Locale, moved: usize, bundles: usize) -> String {
    match locale {
        Locale::En => {
            format!("AI review executed: {moved} targets moved with {bundles} recovery bundles.")
        }
        Locale::ZhCn => format!("AI 审查已执行：移出 {moved} 个目标，写入 {bundles} 个恢复包。"),
    }
}

/// Exact byte count plus a binary-scaled label. Units are locale-neutral.
pub fn clean_bytes_label(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {} ({bytes} B)", UNITS[unit])
}

/// Binary-scaled label without the exact byte suffix. Units stay locale-neutral.
pub fn clean_bytes_short(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

pub fn clean_map_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Disk map",
        Locale::ZhCn => "磁盘地图",
    }
}

pub fn clean_map_subtitle(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Scanned cleanup targets, sized by estimated space. Read only.",
        Locale::ZhCn => "已扫描的清理目标，按估计占用排列。只读。",
    }
}

pub fn clean_map_filtered_note(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "The map follows the current search.",
        Locale::ZhCn => "地图只显示当前搜索结果。",
    }
}

pub fn clean_map_empty(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No sized targets in this view.",
        Locale::ZhCn => "当前视图没有可按占用排列的目标。",
    }
}

pub fn clean_map_recommendations(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Recommendations",
        Locale::ZhCn => "清理建议",
    }
}

pub fn clean_map_recommendations_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Tick a row to include it in review. Preview and confirm still do not move files."
        }
        Locale::ZhCn => "勾选一行只是纳入复查。预览和确认本身不会移动文件。",
    }
}

pub fn clean_map_category(locale: Locale, id: &str) -> &'static str {
    match (locale, id) {
        (Locale::En, "build") => "Build output",
        (Locale::ZhCn, "build") => "构建产物",
        (Locale::En, "cache") => "Safe to clear",
        (Locale::ZhCn, "cache") => "可清理缓存",
        (Locale::En, "agent") => "Agent history",
        (Locale::ZhCn, "agent") => "代理历史",
        (Locale::En, "models") => "AI models",
        (Locale::ZhCn, "models") => "AI 模型",
        (Locale::En, "docker") => "Docker",
        (Locale::ZhCn, "docker") => "Docker",
        (Locale::En, "system") => "System & apps",
        (Locale::ZhCn, "system") => "系统与应用",
        (Locale::En, "other") => "Other",
        (Locale::ZhCn, "other") => "其他",
        (Locale::En, _) => "Other",
        (Locale::ZhCn, _) => "其他",
    }
}

pub fn clean_map_band(locale: Locale, id: &str) -> &'static str {
    match (locale, id) {
        (Locale::En, "recommended") => "Recommended",
        (Locale::ZhCn, "recommended") => "建议清理",
        (Locale::En, "review") => "Needs review",
        (Locale::ZhCn, "review") => "需要复查",
        (Locale::En, "blocked") => "Not selectable",
        (Locale::ZhCn, "blocked") => "不可选",
        (Locale::En, _) => "Needs review",
        (Locale::ZhCn, _) => "需要复查",
    }
}

pub fn clean_map_card_mapped(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Mapped",
        Locale::ZhCn => "已映射",
    }
}

pub fn clean_map_card_recommended(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Recommended",
        Locale::ZhCn => "建议清理",
    }
}

pub fn clean_map_card_review(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Needs review",
        Locale::ZhCn => "需要复查",
    }
}

pub fn clean_map_card_blocked(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Not selectable",
        Locale::ZhCn => "不可选",
    }
}

pub fn clean_map_count(locale: Locale, count: usize) -> String {
    match locale {
        Locale::En => format!("{count} items"),
        Locale::ZhCn => format!("{count} 项"),
    }
}

pub fn clean_map_other(locale: Locale, count: usize) -> String {
    match locale {
        Locale::En => format!("Other · {count}"),
        Locale::ZhCn => format!("其他 · {count} 项"),
    }
}

pub fn clean_map_summary_line(
    locale: Locale,
    bytes: u64,
    count: usize,
    recommended: usize,
    review: usize,
    blocked: usize,
    partial: usize,
) -> String {
    let size = clean_bytes_short(bytes);
    let mut line = match locale {
        Locale::En => {
            let targets = if count == 1 { "target" } else { "targets" };
            format!(
                "Disk map: {size} mapped across {count} {targets}. {recommended} recommended, {review} to review, {blocked} not selectable."
            )
        }
        Locale::ZhCn => format!(
            "磁盘地图：已映射 {size}，共 {count} 个目标。建议清理 {recommended} 个，需要复查 {review} 个，不可选 {blocked} 个。"
        ),
    };
    if partial > 0 {
        match locale {
            Locale::En if partial == 1 => line.push_str(" 1 size is a lower bound."),
            Locale::En => line.push_str(&format!(" {partial} sizes are lower bounds.")),
            Locale::ZhCn => line.push_str(&format!(" {partial} 项大小只是下限。")),
        }
    }
    line
}

pub fn clean_keys_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Keyboard: Tab moves between the Note and the search field; Space toggles the focused row; Alt+Up/Alt+Down move the row cursor; PageUp/PageDown scroll the row window; F7 cycles sort; F8 opens AI review without scanning; F9 scans the saved project folders; Enter previews, then confirms."
        }
        Locale::ZhCn => {
            "键盘：Tab 在备注和搜索框之间移动；空格切换当前行；Alt+↑/Alt+↓ 移动行光标；PageUp/PageDown 滚动行窗口；F7 切换排序；F8 打开 AI 审查且不扫描；F9 扫描已保存的项目文件夹；Enter 先预览，再确认。"
        }
    }
}

pub fn history_clean_executed(
    locale: Locale,
    selected: usize,
    succeeded: usize,
    failed: usize,
    skipped: usize,
    digest: &str,
) -> String {
    match locale {
        Locale::En => format!(
            "Clean executed: {succeeded} succeeded, {failed} failed, {skipped} skipped of {selected} selected targets. Digest {digest}."
        ),
        Locale::ZhCn => format!(
            "清理已执行：所选 {selected} 个目标中 {succeeded} 个成功、{failed} 个失败、{skipped} 个跳过。摘要 {digest}。"
        ),
    }
}

pub fn history_clean_canceled(
    locale: Locale,
    selected: usize,
    succeeded: usize,
    failed: usize,
    skipped: usize,
) -> String {
    match locale {
        Locale::En => format!(
            "Clean canceled: {succeeded} succeeded, {failed} failed, {skipped} skipped of {selected} selected targets."
        ),
        Locale::ZhCn => format!(
            "清理已取消：所选 {selected} 个目标中 {succeeded} 个成功、{failed} 个失败、{skipped} 个跳过。"
        ),
    }
}

pub fn preserved(locale: Locale, store: &str) -> String {
    match locale {
        Locale::En => format!("Saved {store} could not be read. Existing bytes were preserved."),
        Locale::ZhCn => format!("无法读取已保存的{store}。已保留原有字节。"),
    }
}

pub fn store_name(locale: Locale, store: StoreName) -> &'static str {
    match (locale, store) {
        (Locale::En, StoreName::Desktop) => "desktop preferences",
        (Locale::En, StoreName::Language) => "presentation settings",
        (Locale::En, StoreName::ScanRoots) => "scan folders",
        (Locale::En, StoreName::Protections) => "protection list",
        (Locale::ZhCn, StoreName::Desktop) => "桌面设置",
        (Locale::ZhCn, StoreName::Language) => "显示设置",
        (Locale::ZhCn, StoreName::ScanRoots) => "扫描文件夹",
        (Locale::ZhCn, StoreName::Protections) => "保护列表",
    }
}

pub fn software_inventory_summary(
    locale: Locale,
    count: usize,
    selectable: usize,
    manual: usize,
) -> String {
    match locale {
        Locale::En => format!(
            "Software inventory: {count} entries; {selectable} selectable; {manual} manual."
        ),
        Locale::ZhCn => {
            format!("软件清单：{count} 个条目；{selectable} 个可选择；{manual} 个需手动处理。")
        }
    }
}

pub fn software_sources_summary(
    locale: Locale,
    available: usize,
    partial: usize,
    permission: usize,
    unsupported: usize,
) -> String {
    match locale {
        Locale::En => format!(
            "Sources: {available} available; {partial} partial; {permission} permission denied; {unsupported} unsupported."
        ),
        Locale::ZhCn => format!(
            "数据源：{available} 个可用；{partial} 个部分可用；{permission} 个权限受限；{unsupported} 个不受支持。"
        ),
    }
}

pub fn software_empty_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No software inventory",
        Locale::ZhCn => "尚无软件清单",
    }
}

pub fn software_empty_detail(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Inventory is read-only. Only exact eligible current-user MSIX apps can be selected later."
        }
        Locale::ZhCn => "清点过程只读；后续只能选择精确且符合条件的当前用户 MSIX 应用。",
    }
}

pub fn software_preview_summary(locale: Locale, count: usize, digest: &str) -> String {
    match locale {
        Locale::En => format!(
            "Software preview: {count} exact current-user MSIX package(s). Digest: {digest}. Uninstall is irreversible from DevSweep's perspective."
        ),
        Locale::ZhCn => format!(
            "软件预览：{count} 个精确的当前用户 MSIX 包。摘要：{digest}。从 DevSweep 的角度看，卸载不可逆。"
        ),
    }
}

pub fn software_updates_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Available updates",
        Locale::ZhCn => "可用更新",
    }
}

pub fn software_updates_not_checked(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Updates were not checked in this session.",
        Locale::ZhCn => "本次会话尚未检查更新。",
    }
}

pub fn software_updates_unavailable(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Update check unavailable",
        Locale::ZhCn => "无法检查更新",
    }
}

pub fn software_updates_summary(locale: Locale, count: usize) -> String {
    match locale {
        Locale::En => format!("Available updates: {count}"),
        Locale::ZhCn => format!("可用更新：{count}"),
    }
}

pub fn software_startup_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Startup items",
        Locale::ZhCn => "启动项",
    }
}

pub fn software_startup_empty(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No startup items found.",
        Locale::ZhCn => "未找到启动项。",
    }
}

pub fn software_startup_requires_admin(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Requires administrator; view only",
        Locale::ZhCn => "需要管理员权限；仅查看",
    }
}

pub fn software_isolated_executor_unavailable(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No isolated uninstall executor is installed. Host uninstall is refused.",
        Locale::ZhCn => "没有安装隔离卸载执行器。已拒绝卸载主机软件。",
    }
}

pub fn software_digest_missing(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "The live preview digest is missing. Uninstall is refused.",
        Locale::ZhCn => "缺少实时预览摘要。已拒绝卸载。",
    }
}

pub fn optimize_group_name(locale: Locale, group: &str) -> &'static str {
    match (locale, group) {
        (Locale::En, "startup") => "Startup",
        (Locale::ZhCn, "startup") => "启动项",
        (Locale::En, "network_search") => "Network and search",
        (Locale::ZhCn, "network_search") => "网络和搜索",
        (Locale::En, "maintenance") => "Maintenance",
        (Locale::ZhCn, "maintenance") => "维护",
        (Locale::En, _) => "Maintenance",
        (Locale::ZhCn, _) => "维护",
    }
}

pub fn optimize_phase_name(locale: Locale, phase: &str) -> &'static str {
    match (locale, phase) {
        (Locale::En, "landing") => "Landing",
        (Locale::ZhCn, "landing") => "落地",
        (Locale::En, "running") => "Running",
        (Locale::ZhCn, "running") => "运行中",
        (Locale::En, "completed") => "Completed",
        (Locale::ZhCn, "completed") => "已完成",
        (Locale::En, "failed") => "Failed",
        (Locale::ZhCn, "failed") => "失败",
        (Locale::En, "admin_wait") => "Admin wait",
        (Locale::ZhCn, "admin_wait") => "等待管理员",
        (Locale::En, _) => "Landing",
        (Locale::ZhCn, _) => "落地",
    }
}

pub fn optimize_schedule_absent(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "no schedule button",
        Locale::ZhCn => "无调度按钮",
    }
}

pub fn optimize_admin_wait(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Waiting for administrator approval.",
        Locale::ZhCn => "正在等待管理员确认。",
    }
}

pub fn analyze_empty_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No analysis snapshot",
        Locale::ZhCn => "尚无分析快照",
    }
}

pub fn analyze_empty_detail(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Choose a read-only root and start analysis.",
        Locale::ZhCn => "选择只读根目录并开始分析。",
    }
}

pub fn analyze_breadcrumb_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Analysis path breadcrumbs",
        Locale::ZhCn => "分析路径导航",
    }
}

pub fn analyze_treemap_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Current directory treemap",
        Locale::ZhCn => "当前目录矩形树图",
    }
}

pub fn analyze_treemap_description(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Rectangle area represents only observed lower-bound bytes. Unknown-size entries appear in the list only."
        }
        Locale::ZhCn => "矩形面积仅表示已观测的容量下限；容量未知的条目只在列表中显示。",
    }
}

pub fn analyze_list_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Accessible directory list",
        Locale::ZhCn => "无障碍目录列表",
    }
}

pub fn status_health_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Health evidence",
        Locale::ZhCn => "健康证据",
    }
}

pub fn status_health_line(
    locale: Locale,
    subject: &str,
    state: &str,
    reason: &str,
    text: &str,
) -> String {
    match locale {
        Locale::En => format!("{subject} | {state} | {reason} | {text}"),
        Locale::ZhCn => format!("{subject} | {state} | {reason} | {text}"),
    }
}

pub fn status_group_unavailable(locale: Locale, group: &str, reason: &str) -> String {
    match locale {
        Locale::En => format!("{group}: unavailable ({reason})"),
        Locale::ZhCn => format!("{group}：不可用（{reason}）"),
    }
}

pub fn status_group_partial(locale: Locale, group: &str, reasons: &str) -> String {
    match locale {
        Locale::En => format!("{group}: partial ({reasons})"),
        Locale::ZhCn => format!("{group}：部分可用（{reasons}）"),
    }
}

pub fn status_group_permission_denied(locale: Locale, group: &str, reason: &str) -> String {
    match locale {
        Locale::En => format!("{group}: permission denied ({reason})"),
        Locale::ZhCn => format!("{group}：权限被拒绝（{reason}）"),
    }
}

pub fn status_group_unsupported(locale: Locale, group: &str, reason: &str) -> String {
    match locale {
        Locale::En => format!("{group}: unsupported ({reason})"),
        Locale::ZhCn => format!("{group}：不受支持（{reason}）"),
    }
}

pub fn status_live_started(locale: Locale, interval: u32, limit: u32) -> String {
    match locale {
        Locale::En => format!("Live status every {interval}s; process limit {limit}."),
        Locale::ZhCn => format!("实时状态，间隔 {interval} 秒；进程上限 {limit}。"),
    }
}

pub fn status_live_stopped(locale: Locale, reason: &str) -> String {
    match locale {
        Locale::En => format!("Live status stopped ({reason})."),
        Locale::ZhCn => format!("实时状态已停止（{reason}）。"),
    }
}

pub fn status_action_snapshot(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Refresh snapshot",
        Locale::ZhCn => "刷新快照",
    }
}

pub fn status_action_live_start(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Start live",
        Locale::ZhCn => "开始实时",
    }
}

pub fn status_action_live_stop(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Stop live",
        Locale::ZhCn => "停止实时",
    }
}

pub fn status_phase_landing(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Landing",
        Locale::ZhCn => "落地",
    }
}

pub fn status_phase_live(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Live",
        Locale::ZhCn => "实时",
    }
}

pub fn keep_awake_lease(
    locale: Locale,
    state: &str,
    token: &str,
    owner: &str,
    remaining: u64,
    endpoint: &str,
) -> String {
    match locale {
        Locale::En => format!(
            "{state} | token {token} | owner {owner} | remaining {remaining} ms | endpoint {endpoint} | power settings unchanged"
        ),
        Locale::ZhCn => format!(
            "{state} | 令牌 {token} | 所有者 {owner} | 剩余 {remaining} 毫秒 | 端点 {endpoint} | 电源设置未改变"
        ),
    }
}

pub fn clean_screen_active(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Clean Screen covers the window. Escape restores the view.",
        Locale::ZhCn => "清屏覆盖窗口。Escape 恢复视图。",
    }
}

pub fn keep_screen_on_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Keep Screen On",
        Locale::ZhCn => "保持屏幕开启",
    }
}

pub fn clean_screen_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Clean Screen",
        Locale::ZhCn => "清屏",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreName {
    Desktop,
    Language,
    ScanRoots,
    Protections,
}

/// Loading line while a Software segment reads the host.
#[must_use]
pub fn software_loading(locale: Locale, segment: &str) -> &'static str {
    match segment {
        "updates" => pick(
            locale,
            "Checking winget for updates… This is read-only.",
            "正在通过 winget 检查更新…（只读）",
        ),
        "startup" => pick(locale, "Reading startup entries…", "正在读取启动项…"),
        _ => pick(
            locale,
            "Reading installed software… This is read-only.",
            "正在读取已安装软件…（只读）",
        ),
    }
}

/// Chooses one of two literal strings for the active locale. Used by the
/// designed GPUI pages for short labels that have no older catalogue entry.
#[must_use]
pub fn pick(locale: Locale, en: &'static str, zh: &'static str) -> &'static str {
    match locale {
        Locale::En => en,
        Locale::ZhCn => zh,
    }
}

/// Display name of one Optimize catalogue id. Unknown ids show the id.
#[must_use]
pub fn optimize_task_title(locale: Locale, id: &str) -> String {
    let (en, zh) = match id {
        "dns.flush" => ("Flush DNS cache", "刷新 DNS 缓存"),
        "settings.storage_recommendations" => ("Storage recommendations", "存储建议"),
        "settings.search" => ("Search indexing settings", "搜索索引设置"),
        "settings.energy_recommendations" => ("Energy recommendations", "节能建议"),
        "guidance.drive_optimize" => ("Optimize drives", "优化驱动器"),
        "guidance.system_integrity" => ("System file integrity", "系统文件完整性"),
        "guidance.filesystem_check" => ("File system check", "文件系统检查"),
        "guidance.network_reset" => ("Network reset", "网络重置"),
        "maintenance.retrim" => ("Retrim SSD volumes", "SSD 重新修剪"),
        "maintenance.clock_probe" => ("Check clock offset", "检查时钟偏差"),
        "maintenance.clock_sync" => ("Resync system clock", "重新同步系统时钟"),
        "maintenance.image_scan" => ("Scan Windows image", "扫描 Windows 映像"),
        "maintenance.image_repair" => ("Repair Windows image", "修复 Windows 映像"),
        "maintenance.dead_startup" => ("Broken startup entries", "失效的启动项"),
        "maintenance.disk_health" => ("Disk health", "磁盘健康"),
        "maintenance.update_cache" => ("Windows Update cache", "Windows 更新缓存"),
        "maintenance.icon_cache" => ("Icon cache", "图标缓存"),
        "maintenance.search" => ("Rebuild search index", "重建搜索索引"),
        "maintenance.winget_repair" => ("Repair winget sources", "修复 winget 源"),
        other => return other.to_string(),
    };
    pick(locale, en, zh).to_string()
}

/// One-line explanation of one Optimize action class.
#[must_use]
pub fn optimize_class_label(locale: Locale, class: &str) -> &'static str {
    match class {
        "execute" => pick(locale, "Runs after review", "审阅后执行"),
        "settings_handoff" => pick(locale, "Opens Windows Settings", "打开 Windows 设置"),
        "guidance" => pick(locale, "Guidance only", "仅提供指引"),
        _ => pick(locale, "Read-only check", "只读检查"),
    }
}

/// Short description shown under each Clean scope choice.
#[must_use]
pub fn clean_scope_detail(locale: Locale, scope: &str) -> &'static str {
    match scope {
        "project" => pick(
            locale,
            "Build outputs and dependency folders inside the project folders you choose.",
            "你选择的项目文件夹中的构建产物和依赖目录。",
        ),
        _ => pick(
            locale,
            "Package-manager and tool caches in your user profile. Nothing is selected until you review it.",
            "用户目录中的包管理器与工具缓存。审阅之前不会选中任何项。",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_order_copy_is_bilingual_and_offline() {
        let order = ["clean", "software", "optimize", "analyze", "status"];
        let en: Vec<_> = order
            .iter()
            .map(|name| mode_name(Locale::En, name))
            .collect();
        let zh: Vec<_> = order
            .iter()
            .map(|name| mode_name(Locale::ZhCn, name))
            .collect();
        assert_eq!(en, ["Clean", "Software", "Optimize", "Analyze", "Status"]);
        assert_eq!(zh, ["清理", "软件", "优化", "分析", "状态"]);
        for locale in [Locale::En, Locale::ZhCn] {
            for name in order {
                let text = format!(
                    "{} {}",
                    mode_subtitle(locale, name),
                    history_no_replay(locale)
                );
                assert!(!text.contains("http://") && !text.contains("https://"));
            }
        }
    }

    #[test]
    fn chinese_note_placeholder_is_not_english() {
        let en = note_placeholder(Locale::En);
        let zh = note_placeholder(Locale::ZhCn);
        assert_eq!(en, "Type here");
        assert_eq!(zh, "在此输入");
        assert_ne!(en, zh);
    }

    #[test]
    fn clean_workbench_copy_is_bilingual_and_never_claims_success() {
        for locale in [Locale::En, Locale::ZhCn] {
            for text in [
                clean_scan_primary(locale),
                clean_scope_prompt(locale),
                clean_scope_project(locale),
                clean_scope_global(locale),
                clean_global_confirm(locale),
                clean_global_scan_action(locale),
                clean_root_prompt(locale),
                clean_root_choose(locale),
                clean_root_suggestions(locale),
                clean_root_session_only(locale),
                clean_phase_all_clean(locale),
                clean_phase_cleaning(locale),
                clean_phase_completed(locale),
                clean_phase_failed(locale),
                clean_retry_action(locale),
                clean_cancel_action(locale),
                clean_search_placeholder(locale),
                clean_sort_report_order(locale),
                clean_sort_name(locale),
                clean_sort_size(locale),
                clean_select_recommended(locale),
                clean_clear_selection(locale),
                clean_preview_action(locale),
                clean_confirm_action(locale),
                clean_confirm_required_line(locale),
                clean_row_global(locale),
                clean_row_confidence_verified(locale),
                clean_row_confidence_partial(locale),
                clean_row_confidence_unknown(locale),
                clean_row_inspect_only(locale),
                clean_row_protected(locale),
                clean_row_ignored(locale),
                clean_protect_action(locale),
                clean_unprotect_action(locale),
                clean_ignore_action(locale),
                clean_unignore_action(locale),
                clean_protection_session_only(locale),
                clean_ai_title(locale),
                clean_ai_subtitle(locale),
                clean_ai_open_action(locale),
                clean_ai_close_action(locale),
                clean_ai_scan_action(locale),
                clean_ai_landing_prompt(locale),
                clean_ai_phase_scanning(locale),
                clean_ai_phase_cleaning(locale),
                clean_ai_phase_completed(locale),
                clean_ai_phase_failed(locale),
                clean_ai_confirm_action(locale),
                clean_ai_row_selectable(locale),
                clean_ai_row_inspect_only(locale),
                clean_ai_row_protected(locale),
                clean_ai_row_sensitive(locale),
                clean_ai_row_unsupported(locale),
                clean_ai_recovery_title(locale),
                clean_ai_recovery_refresh(locale),
                clean_ai_recovery_empty(locale),
                clean_ai_restore_action(locale),
                clean_ai_restore_refused(locale),
                clean_ai_home_unavailable(locale),
                clean_ai_isolated_home_required(locale),
                clean_ai_keys_hint(locale),
                clean_keys_hint(locale),
            ] {
                assert!(!text.trim().is_empty(), "empty Clean copy: {text:?}");
                let lower = text.to_ascii_lowercase();
                assert!(!lower.contains("http://") && !lower.contains("https://"));
                assert!(
                    !lower.contains("recycle bin"),
                    "no Recycle Bin claim: {text}"
                );
                assert!(
                    !lower.contains("succeed") && !text.contains("成功"),
                    "Clean copy must not claim success: {text}"
                );
            }
            assert!(!clean_phase_scanning(locale, 3).is_empty());
            assert!(!clean_phase_found(locale, 3).is_empty());
            assert!(!clean_root_line(locale, "D:\\work").is_empty());
            assert!(!clean_selection_count(locale, 2, 5).is_empty());
            assert!(!clean_selection_summary(locale, &["a", "b"], 2).is_empty());
            assert!(!clean_preview_digest_line(locale, "sha256:00", 2).is_empty());
            assert!(!clean_result_counts(locale, 1, 2).is_empty());
            assert!(!history_clean_executed(locale, 2, 1, 0, 1, "sha256:00").is_empty());
            assert!(!history_clean_canceled(locale, 2, 1, 0, 1).is_empty());
            assert!(!history_clean_ai_executed(locale, 1, 1).is_empty());
            assert!(!clean_row_rebuild_cost(locale, "python.pytest_cache").is_empty());
            assert!(!clean_row_rebuild_cost(locale, "no.such.rule").is_empty());
            assert!(!clean_ai_provider(locale, "codex").is_empty());
            assert!(!clean_ai_phase_review(locale, 3, 7).is_empty());
            assert!(!clean_ai_category_name(locale, "sensitive").is_empty());
            assert!(
                !clean_ai_category_counts(locale, &[("cache", 2), ("sensitive", 7)]).is_empty()
            );
            assert!(!clean_ai_active_category(locale, "cache", 2).is_empty());
            assert!(!clean_ai_tool_rows(locale, 3).is_empty());
            assert!(!clean_ai_selection_count(locale, 1, 2).is_empty());
            assert!(!clean_ai_other_selections(locale, "cache 1").is_empty());
            assert_ne!(
                clean_row_rebuild_cost(locale, "python.pytest_cache"),
                clean_row_rebuild_cost(locale, "node.node_modules")
            );
            assert!(!clean_ai_digest_line(locale, "sha256:00", 2).is_empty());
            assert!(!clean_ai_row_class(locale, "regenerable_cache").is_empty());
            assert!(!clean_ai_reason(locale, "dirty").is_empty());
            assert_eq!(clean_ai_reason(locale, "zz-unknown"), "zz-unknown");
            assert!(!clean_ai_recovery_line(locale, "id-1", "ab", "cache/regen-a").is_empty());
            assert!(!clean_ai_restore_ready(locale, "cache/regen-a", "ab").is_empty());
        }
        assert_eq!(clean_scan_primary(Locale::En), "Start a Clean scan");
        assert_ne!(
            clean_scan_primary(Locale::En),
            clean_scan_primary(Locale::ZhCn)
        );
        assert_ne!(
            clean_phase_cleaning(Locale::En),
            clean_phase_cleaning(Locale::ZhCn)
        );
        assert_ne!(
            clean_row_protected(Locale::En),
            clean_row_protected(Locale::ZhCn)
        );
        assert_ne!(
            clean_confirm_action(Locale::En),
            clean_confirm_action(Locale::ZhCn)
        );
        assert_ne!(
            clean_sort_report_order(Locale::En),
            clean_sort_report_order(Locale::ZhCn)
        );
        assert_eq!(
            clean_selection_summary(Locale::En, &["a"], 1),
            "Selected: a."
        );
        assert_eq!(
            clean_selection_summary(Locale::En, &["a"], 3),
            "Selected: a and 2 more."
        );
        assert_ne!(
            clean_selection_summary(Locale::En, &["a"], 3),
            clean_selection_summary(Locale::ZhCn, &["a"], 3)
        );
        assert_eq!(clean_bytes_label(0), "0 B");
        assert_eq!(clean_bytes_label(1023), "1023 B");
        assert_eq!(clean_bytes_label(1024), "1.0 KiB (1024 B)");
        assert_eq!(clean_bytes_label(1536), "1.5 KiB (1536 B)");
        assert_eq!(clean_bytes_label(5 * 1024 * 1024), "5.0 MiB (5242880 B)");
        assert_eq!(
            clean_bytes_label(u64::MAX),
            "16777216.0 TiB (18446744073709551615 B)"
        );
    }

    #[test]
    fn clean_keys_hint_names_the_review_chords() {
        let en = clean_keys_hint(Locale::En);
        for needle in [
            "Tab",
            "search field",
            "Space",
            "focused row",
            "Alt+Up",
            "Alt+Down",
            "PageUp",
            "PageDown",
            "row window",
            "F7",
            "sort",
            "F8",
            "F9",
            "Enter",
            "preview",
            "confirm",
        ] {
            assert!(en.contains(needle), "english hint missing {needle}: {en}");
        }

        let zh = clean_keys_hint(Locale::ZhCn);
        for needle in [
            "Tab",
            "搜索框",
            "空格",
            "当前行",
            "Alt+↑",
            "Alt+↓",
            "PageUp",
            "PageDown",
            "行窗口",
            "F7",
            "排序",
            "F9",
            "Enter",
            "预览",
            "确认",
        ] {
            assert!(zh.contains(needle), "chinese hint missing {needle}: {zh}");
        }
        assert_ne!(en, zh);
    }

    #[test]
    fn disk_map_summary_uses_the_singular_for_one_target() {
        let one = clean_map_summary_line(Locale::En, 12, 1, 0, 1, 0, 0);
        assert!(one.contains("1 target."), "{one}");
        assert!(!one.contains("1 targets"), "{one}");
        let many = clean_map_summary_line(Locale::En, 26, 2, 0, 2, 0, 1);
        assert!(many.contains("2 targets."), "{many}");
        assert!(many.contains("1 size is a lower bound."), "{many}");
        let bounds = clean_map_summary_line(Locale::En, 40, 3, 0, 1, 0, 2);
        assert!(bounds.contains("2 sizes are lower bounds."), "{bounds}");
        assert!(!history_unavailable(Locale::En).is_empty());
        assert_ne!(
            history_unavailable(Locale::En),
            history_unavailable(Locale::ZhCn)
        );
        assert_eq!(
            history_operation_line("clean", "op-1", "succeeded"),
            "clean | op-1 | succeeded"
        );
    }
}
