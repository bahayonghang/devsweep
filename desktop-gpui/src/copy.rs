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
        (Locale::En, "analyze") => "Map disk use. This mode is read-only.",
        (Locale::En, "status") => "Host facts from collectors. No score.",
        (Locale::ZhCn, "clean") => "先审查每个清理目标，再移动任何内容。",
        (Locale::ZhCn, "software") => "审查当前用户软件清单后再卸载。",
        (Locale::ZhCn, "optimize") => "审查目录中的操作后再运行。",
        (Locale::ZhCn, "analyze") => "映射磁盘占用。此模式只读。",
        (Locale::ZhCn, "status") => "来自采集器的主机事实。无评分。",
        _ => "",
    }
}

/// The mode chord. `left` and `right` alone stay with the Note's caret, so the
/// shell needs its own chord that the text field does not claim.
pub fn mode_keys_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Keyboard: Alt+Left and Alt+Right switch mode while the Note has focus.",
        Locale::ZhCn => "键盘：焦点在备注输入框时，Alt+← 和 Alt+→ 切换模式。",
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreName {
    Desktop,
    Language,
    ScanRoots,
    Protections,
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
}
