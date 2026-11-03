//! Software and Optimize workbenches. Rows plan and preview through the same
//! Enter activation as the keyboard path; a first activation never confirms.

use super::*;
use crate::optimize_board::OptimizeGroup;
use crate::software_review::{SoftwareFilter, SoftwareSize, SoftwareSort};
use crate::workbench::SoftwareSegment;

impl ShellView {
    pub(super) fn software_page(&mut self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let review = self.session.software_review();
        let segment = review.segment();
        let mut actions = Vec::new();
        if segment.is_some() {
            actions.push(ui::any(
                ui::icon_button(
                    t,
                    "software-refresh",
                    "refresh",
                    copy::pick(locale, "Refresh", "刷新"),
                    ButtonKind::Secondary,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.session.software_refresh();
                    cx.notify();
                })),
            ));
        }
        let segments = [
            (
                SoftwareSegment::Uninstall,
                "uninstall",
                copy::pick(locale, "Uninstall", "卸载"),
            ),
            (
                SoftwareSegment::Updates,
                "updates",
                copy::pick(locale, "Updates", "更新"),
            ),
            (
                SoftwareSegment::Startup,
                "startup",
                copy::pick(locale, "Startup", "启动项"),
            ),
        ]
        .map(|(value, id, label)| {
            ui::segment(
                t,
                ElementId::Name(SharedString::from(format!("software-segment-{id}"))),
                label,
                segment == Some(value),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.session.software_open_segment(value);
                cx.notify();
            }))
        });
        let mut page = div()
            .flex()
            .flex_col()
            .gap(rems(1.25))
            .child(self.page_header(
                t,
                "software",
                copy::mode_name(locale, "software"),
                copy::mode_subtitle(locale, "software"),
                actions,
            ))
            .children(self.shared_banners(t))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(rems(0.75))
                    .child(
                        ui::segmented(t)
                            .id("software-segments")
                            .role(gpui::Role::TabList)
                            .children(segments),
                    )
                    .when(segment.is_some(), |d| {
                        d.child(
                            ui::button(
                                t,
                                "software-close-segment",
                                copy::back_label(locale),
                                ButtonKind::Ghost,
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.session.software_close_segment();
                                cx.notify();
                            })),
                        )
                    }),
            );
        if let Some(notice) = self.session.workbench_notice() {
            page = page.child(ui::banner(
                t,
                Tone::Warning,
                workbench_notice_line(locale, notice),
            ));
        }
        // The landing buttons stay mounted after a segment opens. A UIA
        // client holds the Uninstall button and Invokes it after selecting
        // the tab; unmounting that button returns element-not-available.
        let review_open = segment.is_some();
        page = page.child(self.software_landing(t, review_open, cx));
        if review_open {
            page = page.child(self.software_review_card(t, cx));
        }
        page = page.children(self.workbench_review_card(t));
        page.into_any_element()
    }

    fn software_landing(
        &self,
        t: &Theme,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let locale = self.session.locale();
        let cards = [
            (
                SoftwareSegment::Uninstall,
                "trash",
                copy::pick(locale, "Uninstall", "卸载"),
                copy::pick(
                    locale,
                    "Installed apps from Programs and Features, MSI and Store packages. Removal is planned, previewed, and confirmed one app at a time.",
                    "来自程序和功能、MSI 与应用商店的已安装应用。逐个规划、预览并确认卸载。",
                ),
            ),
            (
                SoftwareSegment::Updates,
                "refresh",
                copy::pick(locale, "Updates", "更新"),
                copy::pick(
                    locale,
                    "Checks winget for newer versions. Checking is read-only and installs nothing.",
                    "通过 winget 检查新版本。检查是只读的，不会安装任何内容。",
                ),
            ),
            (
                SoftwareSegment::Startup,
                "rocket",
                copy::pick(locale, "Startup", "启动项"),
                copy::pick(
                    locale,
                    "Apps that start with Windows. Current-user entries can be turned off and back on.",
                    "随 Windows 启动的应用。当前用户的启动项可以关闭再恢复。",
                ),
            ),
        ]
        .map(|(segment, icon, title, body)| {
            let hover = t.accent;
            let id = match segment {
                SoftwareSegment::Uninstall => "software-card-uninstall",
                SoftwareSegment::Updates => "software-card-updates",
                _ => "software-card-startup",
            };
            let card = div()
                .id(id)
                .accessibility_id(id)
                .role(gpui::Role::Button)
                .aria_label(title)
                .cursor_pointer()
                .border_1()
                .keyboard_focus(t)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.session.software_open_segment(segment);
                    cx.notify();
                }))
                .on_a11y_action(
                    gpui::AccessibleAction::Click,
                    on_view_a11y(cx, move |this, cx| {
                        this.session.software_open_segment(segment);
                        cx.notify();
                    }),
                );
            if compact {
                card.flex()
                    .flex_row()
                    .flex_1()
                    .items_center()
                    .gap(rems(0.45))
                    .h(rems(2.1))
                    .px(rems(0.75))
                    .rounded(rems(0.5))
                    .bg(rgb(t.surface))
                    .border_color(rgb(t.border))
                    .hover(move |s| s.border_color(rgb(hover)))
                    .child(ui::icon(icon, t.text_muted).size(rems(0.9)))
                    .child(div().text_size(rems(0.82)).child(title))
            } else {
                card.flex()
                    .flex_col()
                    .flex_1()
                    .min_w(rems(14.0))
                    .gap(rems(0.6))
                    .p(rems(1.25))
                    .rounded(rems(0.9))
                    .bg(rgb(t.surface))
                    .border_color(rgb(t.border))
                    .hover(move |s| s.border_color(rgb(hover)))
                    .child(ui::icon_tile(t, icon, t.accent))
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
                    .child(ui::muted(t, body))
            }
        });
        div()
            .flex()
            .flex_col()
            .gap(rems(1.0))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap(if compact { rems(0.5) } else { rems(1.0) })
                    .children(cards),
            )
            // A quiet footnote, not an alert: Leftovers is simply out of scope.
            // Hidden once a segment is open so the compact cards stay one row.
            .when(!compact, |landing| {
                landing.child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(rems(0.45))
                        .child(ui::icon("info", t.text_subtle).size(rems(0.85)))
                        .child(
                            div()
                                .text_size(rems(0.8))
                                .text_color(rgb(t.text_subtle))
                                .child(copy::software_leftovers_unavailable(locale)),
                        ),
                )
            })
            .into_any_element()
    }

    fn software_review_card(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let review = self.session.software_review();
        let sort_label = match review.sort() {
            SoftwareSort::Name => copy::pick(locale, "Sort: name", "排序：名称"),
            SoftwareSort::Source => copy::pick(locale, "Sort: source", "排序：来源"),
            SoftwareSort::Version => copy::pick(locale, "Sort: version", "排序：版本"),
            SoftwareSort::Size => copy::pick(locale, "Sort: size", "排序：大小"),
        };
        let count_label = match locale {
            copy::Locale::En => format!("{} of {}", review.visible_count(), review.item_count()),
            copy::Locale::ZhCn => {
                format!("{} / {} 项", review.visible_count(), review.item_count())
            }
        };
        let filter_label = match review.filter() {
            SoftwareFilter::All => copy::pick(locale, "All", "全部"),
            SoftwareFilter::Selectable => copy::pick(locale, "Removable", "可卸载"),
            SoftwareFilter::Protected => copy::pick(locale, "Protected", "受保护"),
            SoftwareFilter::PermissionDenied => copy::pick(locale, "Needs admin", "需要管理员"),
        };
        let toolbar = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap(rems(0.5))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(rems(0.5))
                    .w(rems(20.0))
                    .child(ui::icon("search", t.text_subtle))
                    .child(div().flex_1().child(self.search.clone())),
            )
            .child(div().flex_1())
            .when(review.item_count() > 0, |bar| {
                bar.child(ui::badge(t, count_label, Tone::Neutral))
            })
            .child(
                ui::icon_button(t, "software-sort", "sort", sort_label, ButtonKind::Ghost)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.session.software_cycle_sort();
                        cx.notify();
                    })),
            )
            .child(
                ui::icon_button(
                    t,
                    "software-filter",
                    "rules",
                    filter_label,
                    ButtonKind::Ghost,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.session.software_cycle_filter();
                    cx.notify();
                })),
            );
        let has_items = review.item_count() > 0;
        let summary = review
            .lines()
            .iter()
            .filter(|_| has_items)
            .map(|line| ui::muted(t, line.clone()));
        let cursor = self
            .session
            .shell()
            .selection(Mode::Software)
            .map(str::to_string);
        let pending = review.pending_id().map(str::to_string);
        let mut rows = div()
            .id("software-rows")
            .role(gpui::Role::List)
            .flex()
            .flex_col()
            .gap(rems(0.15));
        let mut any_row = false;
        for id in self.session.shell().visible_row_ids(Mode::Software) {
            let Some(item) = review.item(id) else {
                continue;
            };
            any_row = true;
            let focused = cursor.as_deref() == Some(id.as_str());
            let text = review.row_line(id, locale, focused).unwrap_or_default();
            let needs_admin =
                item.permission.contains("permission") || item.permission.contains("administrator");
            // Each segment states what the row is: an app's uninstall
            // authority, an update's exact versions, or a startup entry's
            // state and who may change it.
            let (status, tone, subtitle) = match review.segment() {
                Some(SoftwareSegment::Updates) => (
                    copy::pick(locale, "Update available", "有可用更新"),
                    Tone::Info,
                    format!(
                        "{} → {} · {}",
                        item.installed_version.as_deref().unwrap_or("?"),
                        item.version,
                        item.source
                    ),
                ),
                Some(SoftwareSegment::Startup) => {
                    let (label, tone) = match item.version.as_str() {
                        "enabled" => (copy::pick(locale, "Enabled", "已启用"), Tone::Success),
                        "disabled" => (copy::pick(locale, "Disabled", "已禁用"), Tone::Neutral),
                        _ => (copy::pick(locale, "Unknown", "未知"), Tone::Warning),
                    };
                    let authority = if item.permission.contains("protected") {
                        copy::pick(
                            locale,
                            "protected, DevSweep leaves it",
                            "受保护，DevSweep 不改动",
                        )
                    } else if needs_admin {
                        copy::pick(locale, "needs admin to change", "需要管理员才能更改")
                    } else if item.permission == "allowed" {
                        copy::pick(locale, "you can change this", "当前用户可更改")
                    } else {
                        copy::pick(locale, "review only", "仅可审阅")
                    };
                    let location = copy::startup_location_label(locale, &item.source);
                    (label, tone, format!("{location} · {authority}"))
                }
                _ => {
                    let (label, tone) = if item.permission.contains("protected") {
                        (copy::pick(locale, "Protected", "受保护"), Tone::Neutral)
                    } else if needs_admin {
                        (
                            copy::pick(locale, "Needs admin", "需要管理员"),
                            Tone::Warning,
                        )
                    } else if item.selectable {
                        (copy::pick(locale, "Removable", "可卸载"), Tone::Success)
                    } else {
                        (copy::pick(locale, "Review only", "仅可审阅"), Tone::Neutral)
                    };
                    (label, tone, format!("{} · {}", item.version, item.source))
                }
            };
            let element_id = SharedString::from(format!("software-row-{id}"));
            let select_id = id.clone();
            let review_id = id.clone();
            let awaiting = pending.as_deref() == Some(id.as_str());
            let initial = item
                .name
                .chars()
                .next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_default();
            rows = rows.child(
                ui::list_row(t, ElementId::Name(element_id.clone()), focused, focused)
                    .accessibility_id(element_id)
                    .aria_label(SharedString::from(text))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.session
                            .select_mode_row(Mode::Software, select_id.clone());
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex()
                            .flex_shrink_0()
                            .items_center()
                            .justify_center()
                            .size(rems(2.0))
                            .rounded(rems(0.5))
                            .bg(t.tint(t.accent, 0x22))
                            .text_color(rgb(t.accent))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(initial),
                    )
                    .child(ui::two_line(t, item.name.clone(), subtitle))
                    .when_some(item.size, |row, size| {
                        // The qualifier always travels with the number.
                        let (prefix, note) = match size {
                            SoftwareSize::Estimate(_) => {
                                ("~", copy::pick(locale, "reported", "安装程序报告"))
                            }
                            SoftwareSize::Measured(_) => {
                                ("", copy::pick(locale, "measured", "实测"))
                            }
                            SoftwareSize::AtLeast(_) => {
                                ("≥ ", copy::pick(locale, "partial", "部分"))
                            }
                        };
                        row.child(
                            div()
                                .flex()
                                .flex_col()
                                .items_end()
                                .flex_shrink_0()
                                .child(
                                    div()
                                        .text_size(rems(0.86))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(format!("{prefix}{}", ui::bytes(size.bytes()))),
                                )
                                .child(
                                    div()
                                        .text_size(rems(0.72))
                                        .text_color(rgb(t.text_subtle))
                                        .child(note),
                                ),
                        )
                    })
                    .child(ui::badge(t, status, tone))
                    .when(focused, |row| {
                        row.child(
                            ui::button(
                                t,
                                "software-row-review",
                                if awaiting {
                                    copy::pick(locale, "Confirm", "确认")
                                } else {
                                    copy::pick(locale, "Review", "审阅")
                                },
                                if awaiting {
                                    ButtonKind::Danger
                                } else {
                                    ButtonKind::Secondary
                                },
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.session
                                        .activate_mode_row(Mode::Software, review_id.clone());
                                    cx.notify();
                                },
                            )),
                        )
                    }),
            );
        }
        if review.loading() {
            rows = rows.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(rems(0.75))
                    .py(rems(1.5))
                    .child(ui::muted(t, review.lines().join(" ")))
                    .child(ui::progress_track(t)),
            );
        } else if !any_row && !review.lines().is_empty() {
            let lines = review.lines();
            rows = rows.child(ui::empty_state(
                t,
                "software",
                lines[0].clone(),
                lines[1..].join(" "),
            ));
        } else if !any_row {
            let (title, body) = match review.segment() {
                Some(SoftwareSegment::Updates) => (
                    copy::software_updates_title(locale),
                    copy::software_updates_not_checked(locale),
                ),
                Some(SoftwareSegment::Startup) => (
                    copy::software_startup_title(locale),
                    copy::software_startup_empty(locale),
                ),
                _ => (
                    copy::software_empty_title(locale),
                    copy::software_empty_detail(locale),
                ),
            };
            rows = rows.child(ui::empty_state(t, "software", title, body));
        }
        ui::card(t)
            .gap(rems(0.9))
            .child(toolbar)
            .child(div().flex().flex_col().gap(rems(0.2)).children(summary))
            .child(ui::divider(t))
            .child(rows)
            .into_any_element()
    }

    /// Domain review lines from the last activation, shown as a review panel.
    fn workbench_review_card(&self, t: &Theme) -> Option<gpui::AnyElement> {
        let locale = self.session.locale();
        let lines = self.session.workbench_review_lines();
        if lines.is_empty() {
            return None;
        }
        Some(ui::any(
            ui::card(t)
                .id("workbench-review")
                .role(gpui::Role::Group)
                .aria_label(copy::pick(locale, "Review", "审阅"))
                .gap(rems(0.5))
                .child(ui::card_header(t, copy::pick(locale, "Review", "审阅")))
                .children(lines.iter().map(|line| {
                    div()
                        .text_size(rems(0.82))
                        .text_color(rgb(t.text_muted))
                        .child(line.clone())
                })),
        ))
    }

    pub(super) fn optimize_page(&mut self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let board = self.session.optimize_board();
        let phase = board.phase();
        let phase_tone = match phase {
            crate::optimize_board::OptimizePhase::Landing => Tone::Neutral,
            crate::optimize_board::OptimizePhase::Running => Tone::Accent,
            crate::optimize_board::OptimizePhase::Completed => Tone::Success,
            crate::optimize_board::OptimizePhase::Failed => Tone::Danger,
            crate::optimize_board::OptimizePhase::AdminWait => Tone::Warning,
        };
        let mut page = div()
            .flex()
            .flex_col()
            .gap(rems(1.25))
            .child(self.page_header(
                t,
                "optimize",
                copy::mode_name(locale, "optimize"),
                copy::mode_subtitle(locale, "optimize"),
                if phase == crate::optimize_board::OptimizePhase::Landing {
                    Vec::new()
                } else {
                    vec![ui::any(ui::badge(
                        t,
                        copy::optimize_phase_name(locale, phase.key()),
                        phase_tone,
                    ))]
                },
            ))
            .children(self.shared_banners(t));
        if phase == crate::optimize_board::OptimizePhase::AdminWait {
            page = page.child(ui::banner(
                t,
                Tone::Warning,
                copy::optimize_admin_wait(locale),
            ));
        }
        if let Some(notice) = self.session.workbench_notice() {
            page = page.child(ui::banner(
                t,
                Tone::Warning,
                workbench_notice_line(locale, notice),
            ));
        }
        let cursor = self
            .session
            .shell()
            .selection(Mode::Optimize)
            .map(str::to_string);
        let pending = self.session.workbench_pending_id().map(str::to_string);
        let open_group = board.group();
        let groups = OptimizeGroup::ALL
            .into_iter()
            .filter(|group| open_group.is_none_or(|open| open == *group));
        let mut grid = div().flex().flex_col().gap(rems(1.0));
        for group in groups {
            let ids = crate::optimize_board::ids_in(group);
            let icon = match group {
                OptimizeGroup::Startup => "rocket",
                OptimizeGroup::NetworkAndSearch => "network",
                OptimizeGroup::Maintenance => "wrench",
            };
            let group_id = match group {
                OptimizeGroup::Startup => crate::optimize_board::GROUP_STARTUP,
                OptimizeGroup::NetworkAndSearch => crate::optimize_board::GROUP_NETWORK,
                OptimizeGroup::Maintenance => crate::optimize_board::GROUP_MAINTENANCE,
            };
            let mut rows = div()
                .id(ElementId::Name(SharedString::from(format!(
                    "optimize-rows-{}",
                    group.key()
                ))))
                .role(gpui::Role::List)
                .flex()
                .flex_col()
                .gap(rems(0.15));
            for id in &ids {
                rows = rows.child(self.optimize_row(
                    t,
                    id,
                    cursor.as_deref() == Some(id.as_str()),
                    pending.as_deref() == Some(id.as_str()),
                    cx,
                ));
            }
            let header_action = if open_group.is_some() {
                ui::button(
                    t,
                    "optimize-close-group",
                    copy::back_label(locale),
                    ButtonKind::Ghost,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.session.close_mode_detail();
                    cx.notify();
                }))
            } else {
                ui::icon_button(
                    t,
                    ElementId::Name(SharedString::from(format!("optimize-open-{}", group.key()))),
                    "chevron-right",
                    copy::pick(locale, "Focus", "聚焦"),
                    ButtonKind::Ghost,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.session.activate_mode_row(Mode::Optimize, group_id);
                    cx.notify();
                }))
            };
            grid = grid.child(
                ui::card(t)
                    .id(ElementId::Name(SharedString::from(format!(
                        "optimize-group-{}",
                        group.key()
                    ))))
                    .role(gpui::Role::Group)
                    .aria_label(copy::optimize_group_name(locale, group.key()))
                    .gap(rems(0.75))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(rems(0.75))
                            .child(ui::icon_tile(t, icon, t.accent))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .child(
                                        div()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(copy::optimize_group_name(locale, group.key())),
                                    )
                                    .child(ui::muted(
                                        t,
                                        match locale {
                                            copy::Locale::En if ids.len() == 1 => {
                                                "1 task".to_string()
                                            }
                                            copy::Locale::En => format!("{} tasks", ids.len()),
                                            copy::Locale::ZhCn => format!("{} 项任务", ids.len()),
                                        },
                                    )),
                            )
                            .child(header_action),
                    )
                    .child(ui::divider(t))
                    .child(rows),
            );
        }
        page = page.child(grid);
        page = page.children(self.workbench_review_card(t));
        page.into_any_element()
    }

    fn optimize_row(
        &self,
        t: &Theme,
        id: &str,
        focused: bool,
        awaiting: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let locale = self.session.locale();
        let entry = devsweep_core::optimize::catalogue_entry(id);
        let class = entry.map(|entry| match entry.action_class {
            devsweep_core::optimize::MaintenanceActionClass::Execute => "execute",
            devsweep_core::optimize::MaintenanceActionClass::SettingsHandoff => "settings_handoff",
            devsweep_core::optimize::MaintenanceActionClass::Guidance => "guidance",
            devsweep_core::optimize::MaintenanceActionClass::Observe => "observe",
        });
        let class = class.unwrap_or("observe");
        let (tone, icon) = match class {
            "execute" => (Tone::Accent, "play"),
            "settings_handoff" => (Tone::Info, "external"),
            "guidance" => (Tone::Neutral, "info"),
            _ => (Tone::Neutral, "eye"),
        };
        let schedulable = crate::optimize_board::schedule_allowed(id);
        let text = self
            .session
            .optimize_board()
            .row_line(id, locale, focused)
            .unwrap_or_else(|| id.to_string());
        let element_id = SharedString::from(format!("optimize-row-{id}"));
        let select_id = id.to_string();
        let review_id = id.to_string();
        ui::list_row(t, ElementId::Name(element_id.clone()), focused, focused)
            .accessibility_id(element_id)
            .aria_label(SharedString::from(text))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.session
                    .select_mode_row(Mode::Optimize, select_id.clone());
                cx.notify();
            }))
            .child(ui::icon(icon, t.tone(tone)))
            .child(ui::two_line(
                t,
                copy::optimize_task_title(locale, id),
                copy::optimize_class_label(locale, class),
            ))
            .when(!schedulable, |row| {
                row.child(ui::badge(
                    t,
                    copy::optimize_schedule_absent(locale),
                    Tone::Neutral,
                ))
            })
            .child(ui::badge(t, copy::workbench_not_run(locale), Tone::Neutral))
            .when(focused, |row| {
                row.child(
                    ui::button(
                        t,
                        "optimize-row-review",
                        if awaiting {
                            copy::pick(locale, "Confirm", "确认")
                        } else {
                            copy::pick(locale, "Review", "审阅")
                        },
                        if awaiting {
                            ButtonKind::Danger
                        } else {
                            ButtonKind::Secondary
                        },
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.session
                            .activate_mode_row(Mode::Optimize, review_id.clone());
                        cx.notify();
                    })),
                )
            })
            .into_any_element()
    }
}
