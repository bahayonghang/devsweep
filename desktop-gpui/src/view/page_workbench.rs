//! Software and Optimize workbenches. Rows plan and preview through the same
//! Enter activation as the keyboard path; a first activation never confirms.

use super::*;
use crate::optimize_board::OptimizeGroup;
use crate::software_review::{SoftwareFilter, SoftwareSize, SoftwareSort};
use crate::workbench::SoftwareSegment;

impl ShellView {
    pub(super) fn software_page(
        &mut self,
        t: &Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let locale = self.session.locale();
        let segment = self.session.software_review().segment();
        let loading = self.session.software_review().loading();
        let refresh = ui::icon_button(
            t,
            "software-refresh",
            "refresh",
            copy::pick(locale, "Refresh", "刷新"),
            if loading {
                ButtonKind::Disabled
            } else {
                ButtonKind::Secondary
            },
        )
        .accessibility_id("software-refresh")
        .on_click(cx.listener(|this, _, _, cx| {
            this.session.software_refresh();
            cx.notify();
        }))
        .on_a11y_action(
            gpui::AccessibleAction::Click,
            on_view_a11y(cx, |this, cx| {
                this.session.software_refresh();
                cx.notify();
            }),
        );
        let tabs = [
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
            let id = SharedString::from(format!("software-segment-{id}"));
            ui::segment(
                t,
                ElementId::Name(id.clone()),
                label,
                segment == Some(value),
            )
            .accessibility_id(id)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.session.software_open_segment(value);
                cx.notify();
            }))
            .on_a11y_action(
                gpui::AccessibleAction::Click,
                on_view_a11y(cx, move |this, cx| {
                    this.session.software_open_segment(value);
                    cx.notify();
                }),
            )
        });
        let review = self.session.software_review();
        let status = if loading {
            Some(
                review
                    .lines()
                    .iter()
                    .map(|line| copy::software_review_line(locale, line))
                    .collect::<Vec<_>>()
                    .join(" "),
            )
        } else if review.item_count() > 0 {
            Some(match locale {
                copy::Locale::En => {
                    format!("{} of {}", review.visible_count(), review.item_count())
                }
                copy::Locale::ZhCn => {
                    format!("{} / {} 项", review.visible_count(), review.item_count())
                }
            })
        } else {
            Some(copy::software_empty_status(locale).to_string())
        };
        let bar = page_bar(t, status, None, vec![ui::any(refresh)]);
        let mut page = div()
            .flex()
            .flex_col()
            .gap(ui::SPACE_3)
            .children(self.shared_banners(t))
            .child(
                div().flex().child(
                    ui::segmented(t)
                        .id("software-segments")
                        .role(gpui::Role::TabList)
                        .aria_label(copy::pick(locale, "Software tasks", "软件任务"))
                        .children(tabs),
                ),
            );
        if let Some(notice) = self.session.workbench_notice() {
            page = page.child(ui::banner(
                t,
                Tone::Warning,
                workbench_notice_line(locale, notice),
            ));
        }
        let row_height =
            (window.viewport_size().height - window.rem_size() * 23.0).max(window.rem_size() * 8.0);
        sheet_frame(
            bar,
            page.child(self.software_review_card(t, row_height, cx))
                .into_any_element(),
            None,
        )
    }

    fn software_review_card(
        &self,
        t: &Theme,
        row_height: gpui::Pixels,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let locale = self.session.locale();
        let review = self.session.software_review();
        let sort_label = match review.sort() {
            SoftwareSort::Name => copy::pick(locale, "Sort: name", "排序：名称"),
            SoftwareSort::Source => copy::pick(locale, "Sort: source", "排序：来源"),
            SoftwareSort::Version => copy::pick(locale, "Sort: version", "排序：版本"),
            SoftwareSort::Size => copy::pick(locale, "Sort: size", "排序：大小"),
            SoftwareSort::InstalledOn => {
                copy::pick(locale, "Sort: installed / serviced", "排序：安装/维护日期")
            }
        };
        let count_label = match locale {
            copy::Locale::En => format!("{} of {}", review.visible_count(), review.item_count()),
            copy::Locale::ZhCn => {
                format!("{} / {} 项", review.visible_count(), review.item_count())
            }
        };
        let filter_label = match review.filter() {
            SoftwareFilter::All => copy::pick(locale, "All", "全部"),
            SoftwareFilter::Selectable => copy::pick(locale, "Eligible for review", "可审阅"),
            SoftwareFilter::Protected => copy::pick(locale, "Protected", "受保护"),
            SoftwareFilter::PermissionDenied => copy::pick(locale, "Needs admin", "需要管理员"),
        };
        let toolbar = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap(ui::SPACE_1)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(ui::SPACE_1)
                    .w(ui::SPACE_6 * 5.0)
                    .max_w_full()
                    .child(ui::icon("search", t.text_muted))
                    .child(div().flex_1().min_w_0().child(self.search.clone())),
            )
            .child(div().flex_1())
            .when(review.item_count() > 0, |bar| {
                bar.child(ui::badge(t, count_label, Tone::Neutral))
            })
            .child(
                ui::icon_button(t, "software-sort", "sort", sort_label, ButtonKind::Ghost)
                    .accessibility_id("software-sort")
                    .on_click(cx.listener(|this, _, _, cx| {
                        if this.session.software_review().segment()
                            == Some(SoftwareSegment::Uninstall)
                        {
                            this.software_sort_open = !this.software_sort_open;
                        } else {
                            this.session.software_cycle_sort();
                        }
                        cx.notify();
                    }))
                    .on_a11y_action(
                        gpui::AccessibleAction::Click,
                        on_view_a11y(cx, |this, cx| {
                            if this.session.software_review().segment()
                                == Some(SoftwareSegment::Uninstall)
                            {
                                this.software_sort_open = !this.software_sort_open;
                            } else {
                                this.session.software_cycle_sort();
                            }
                            cx.notify();
                        }),
                    ),
            )
            .when(
                review.segment() == Some(SoftwareSegment::Uninstall),
                |bar| {
                    bar.child(
                        ui::button(
                            t,
                            "software-source-filter",
                            review.source_filter().label(locale),
                            ButtonKind::Ghost,
                        )
                        .accessibility_id("software-source-filter")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.session.software_cycle_source_filter();
                            cx.notify();
                        }))
                        .on_a11y_action(
                            gpui::AccessibleAction::Click,
                            on_view_a11y(cx, |this, cx| {
                                this.session.software_cycle_source_filter();
                                cx.notify();
                            }),
                        ),
                    )
                },
            )
            .child(
                ui::icon_button(
                    t,
                    "software-filter",
                    "rules",
                    filter_label,
                    ButtonKind::Ghost,
                )
                .accessibility_id("software-filter")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.session.software_cycle_filter();
                    cx.notify();
                }))
                .on_a11y_action(
                    gpui::AccessibleAction::Click,
                    on_view_a11y(cx, |this, cx| {
                        this.session.software_cycle_filter();
                        cx.notify();
                    }),
                ),
            );
        let has_items = review.item_count() > 0;
        let summary = review
            .lines()
            .iter()
            .filter(|_| has_items)
            .map(|line| ui::muted(t, copy::software_review_line(locale, line)));
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
            .flex_col();
        let mut any_row = false;
        for id in self.session.shell().visible_row_ids(Mode::Software) {
            let Some(item) = review.item(id) else {
                continue;
            };
            any_row = true;
            let focused = cursor.as_deref() == Some(id.as_str());
            let text = review.accessible_row(id, locale).unwrap_or_default();
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
                        copy::software_source_label(locale, &item.source)
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
                        copy::pick(locale, "change can be previewed", "可预览更改")
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
                        (
                            copy::pick(locale, "Eligible for review", "可审阅"),
                            Tone::Success,
                        )
                    } else {
                        (copy::pick(locale, "Review only", "仅可审阅"), Tone::Neutral)
                    };
                    (
                        label,
                        tone,
                        format!(
                            "{} · {} · {}",
                            copy::software_version_label(locale, &item.version),
                            item.publisher.as_deref().unwrap_or(copy::pick(
                                locale,
                                "Publisher unknown",
                                "发布者未知"
                            )),
                            copy::software_source_label(locale, &item.source)
                        ),
                    )
                }
            };
            let element_id = SharedString::from(format!("software-row-{id}"));
            let select_id = id.clone();
            let select_access_id = id.clone();
            let review_id = id.clone();
            let review_access_id = id.clone();
            let related_id = id.clone();
            let related_access_id = id.clone();
            let awaiting = pending.as_deref() == Some(id.as_str());
            let preview_only = awaiting && review.host_backed();
            let initial = item
                .name
                .chars()
                .next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_default();
            let avatar = if let Some(image) = self.session.software_icon(id) {
                div()
                    .flex_shrink_0()
                    .size(ui::SPACE_5)
                    .child(img(image).size_full())
                    .into_any_element()
            } else {
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .justify_center()
                    .size(ui::SPACE_5)
                    .rounded(ui::RADIUS_ITEM)
                    .bg(t.tint(t.accent, 0x22))
                    .text_color(rgb(t.accent))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(initial)
                    .into_any_element()
            };
            rows = rows.child(
                ui::list_row(t, ElementId::Name(element_id.clone()), focused, focused)
                    .flex_wrap()
                    .accessibility_id(element_id)
                    .aria_label(SharedString::from(text))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.session
                            .select_mode_row(Mode::Software, select_id.clone());
                        cx.notify();
                    }))
                    .on_a11y_action(
                        gpui::AccessibleAction::Click,
                        on_view_a11y(cx, move |this, cx| {
                            this.session
                                .select_mode_row(Mode::Software, select_access_id.clone());
                            cx.notify();
                        }),
                    )
                    .child(avatar)
                    .child(ui::two_line(t, item.name.clone(), subtitle))
                    .when(
                        review.segment() == Some(SoftwareSegment::Uninstall),
                        |row| {
                            row.child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_end()
                                    .flex_shrink_0()
                                    .text_size(ui::TYPE_META)
                                    .text_color(rgb(t.text_muted))
                                    .child(item.installed_on.clone().unwrap_or_else(|| {
                                        copy::pick(locale, "Date unknown", "日期未知").to_string()
                                    }))
                                    .child(if item.source == "msix_current_user" {
                                        copy::pick(
                                            locale,
                                            "Installed / updated (UTC)",
                                            "安装/更新（UTC）",
                                        )
                                    } else {
                                        copy::pick(locale, "Installed / serviced", "安装/维护日期")
                                    }),
                            )
                        },
                    )
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
                                        .text_size(ui::TYPE_BODY)
                                        .font_features(ui::tabular_numerals())
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(format!("{prefix}{}", ui::bytes(size.bytes()))),
                                )
                                .child(
                                    div()
                                        .text_size(ui::TYPE_META)
                                        .text_color(rgb(t.text_muted))
                                        .child(note),
                                ),
                        )
                    })
                    .when(
                        item.size.is_none() && review.segment() == Some(SoftwareSegment::Uninstall),
                        |row| {
                            row.child(ui::muted(t, copy::pick(locale, "Size unknown", "容量未知")))
                        },
                    )
                    .child(ui::badge(t, status, tone))
                    .when(focused, |row| {
                        row.child(
                            div()
                                .w_full()
                                .flex()
                                .justify_end()
                                .gap(ui::SPACE_1)
                                .when(
                                    review.segment() == Some(SoftwareSegment::Uninstall),
                                    |row| {
                                        row.child(
                                            ui::button(
                                                t,
                                                "software-related",
                                                copy::pick(
                                                    locale,
                                                    "Related directories",
                                                    "相关目录",
                                                ),
                                                ButtonKind::Secondary,
                                            )
                                            .accessibility_id("software-related")
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.session.software_preview_related(&related_id);
                                                cx.notify();
                                            }))
                                            .on_a11y_action(
                                                gpui::AccessibleAction::Click,
                                                on_view_a11y(cx, move |this, cx| {
                                                    this.session.software_preview_related(
                                                        &related_access_id,
                                                    );
                                                    cx.notify();
                                                }),
                                            ),
                                        )
                                    },
                                )
                                .child(
                                    ui::button(
                                        t,
                                        "software-row-review",
                                        if preview_only {
                                            copy::pick(locale, "Preview only", "仅预览")
                                        } else if awaiting {
                                            copy::pick(locale, "Confirm", "确认")
                                        } else {
                                            copy::pick(locale, "Review", "审阅")
                                        },
                                        if preview_only {
                                            ButtonKind::Disabled
                                        } else if awaiting {
                                            ButtonKind::Danger
                                        } else {
                                            ButtonKind::Secondary
                                        },
                                    )
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if !preview_only {
                                            this.session.activate_mode_row(
                                                Mode::Software,
                                                review_id.clone(),
                                            );
                                        }
                                        cx.notify();
                                    }))
                                    .on_a11y_action(
                                        gpui::AccessibleAction::Click,
                                        on_view_a11y(cx, move |this, cx| {
                                            if !preview_only {
                                                this.session.activate_mode_row(
                                                    Mode::Software,
                                                    review_access_id.clone(),
                                                );
                                            }
                                            cx.notify();
                                        }),
                                    ),
                                ),
                        )
                    }),
            );
        }
        if review.loading() {
            // The page bar already says what is being read.
            rows = rows.child(div().py(ui::SPACE_4).child(ui::progress_track(t)));
        } else if !any_row && review.item_count() > 0 {
            rows = rows.child(ui::empty_state(
                t,
                "search",
                copy::pick(locale, "No matching items", "没有匹配项目"),
                copy::pick(
                    locale,
                    "Change the search or filter to see more items.",
                    "调整搜索或筛选条件以显示更多项目。",
                ),
            ));
        } else if !any_row && !review.lines().is_empty() {
            let lines = review
                .lines()
                .iter()
                .map(|line| copy::software_review_line(locale, line))
                .collect::<Vec<_>>();
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
            .gap(ui::SPACE_3)
            .child(toolbar)
            .when(self.software_sort_open && review.segment() == Some(SoftwareSegment::Uninstall), |card| {
                card.child(div().id("software-sort-options").role(gpui::Role::Group)
                    .aria_label(copy::pick(locale, "Sort software", "软件排序"))
                    .flex().flex_wrap().gap(ui::SPACE_1)
                    .children([
                        (SoftwareSort::Name, "name", copy::pick(locale, "Name", "名称")),
                        (SoftwareSort::Size, "size", copy::pick(locale, "Size", "容量")),
                        (SoftwareSort::InstalledOn, "installed", copy::pick(locale, "Installed / serviced", "安装/维护日期")),
                    ].into_iter().map(|(sort, id, label)| {
                        let id = SharedString::from(format!("software-sort-{id}"));
                        ui::button(t, ElementId::Name(id.clone()), label,
                            if review.sort() == sort { ButtonKind::Primary } else { ButtonKind::Secondary })
                            .accessibility_id(id)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.session.software_set_sort(sort); this.software_sort_open = false; cx.notify();
                            }))
                            .on_a11y_action(gpui::AccessibleAction::Click, on_view_a11y(cx, move |this, cx| {
                                this.session.software_set_sort(sort); this.software_sort_open = false; cx.notify();
                            }))
                    })))
            })
            .child(ui::muted(t, match review.segment() {
                Some(SoftwareSegment::Updates) => copy::pick(locale, "Checks for updates only; nothing is installed.", "仅检查更新，不会安装任何内容。"),
                Some(SoftwareSegment::Startup) => copy::pick(locale, "Review startup entries and preview changes. Applying host changes is unavailable here.", "审阅启动项并预览更改；此处不能修改本机启动项。"),
                _ => copy::pick(locale, "Review uninstall eligibility and preview a plan. Host uninstall is unavailable here.", "审阅卸载条件并预览计划；此处不能卸载本机应用。"),
            }))
            .child(div().flex().flex_col().children(summary))
            .when_some(self.software_related_panel(t, cx), |card, panel| card.child(panel))
            .child(ui::divider(t))
            .child(rows.max_h(if review.related().is_some() { row_height * 0.55 } else { row_height }).overflow_y_scroll())
            .into_any_element()
    }

    fn software_related_panel(
        &self,
        t: &Theme,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let locale = self.session.locale();
        let related = self.session.software_review().related()?;
        let mut panel = div().id("software-related-preview").role(gpui::Role::Group)
            .aria_label(copy::pick(locale, "Related directories, preview only", "相关目录，仅预览"))
            .flex().flex_col().gap(ui::SPACE_1)
            .child(div().flex().items_center().gap(ui::SPACE_1)
                .child(div().text_size(ui::TYPE_TITLE).font_weight(FontWeight::SEMIBOLD)
                    .child(format!("{} · {}", related.name, copy::pick(locale, "Related directories, preview only", "相关目录 · 仅预览"))))
                .child(div().flex_1())
                .child(ui::button(t, "software-related-close", copy::pick(locale, "Close", "关闭"), ButtonKind::Ghost)
                    .accessibility_id("software-related-close")
                    .on_click(cx.listener(|this, _, _, cx| { this.session.software_close_related(); cx.notify(); }))
                    .on_a11y_action(gpui::AccessibleAction::Click, on_view_a11y(cx, |this, cx| {
                        this.session.software_close_related(); cx.notify();
                    }))))
            .child(ui::muted(t, copy::pick(locale,
                "These directories may still be in use. Review candidates after uninstall; nothing is selected for removal.",
                "这些目录可能仍在使用。卸载后再审阅候选；此处不会选择或移除任何内容。")));
        panel = match &related.result {
            None => panel.child(ui::muted(t, copy::pick(locale, "Reading related directories…", "正在读取相关目录…"))),
            Some(Err(reason)) => panel.child(ui::muted(t, match reason.as_str() {
                "protection_unavailable" => copy::pick(locale,
                    "The protection list could not be read. Related directories were not scanned.",
                    "无法读取保护清单，未扫描相关目录。"),
                "unknown_selection" | "invalid_identity" => copy::pick(locale,
                    "The application identity changed. Refresh the inventory before trying again.",
                    "应用身份已变化，请刷新软件清单后重试。"),
                _ => copy::pick(locale,
                    "Related directories are unavailable. No conclusion about absence can be drawn.",
                    "无法读取相关目录，不能据此认定没有候选目录。"),
            })),
            Some(Ok(preview)) => {
                let candidates = preview.apps.iter().flat_map(|app| &app.candidates).collect::<Vec<_>>();
                if candidates.is_empty() {
                    panel.child(ui::muted(t, copy::pick(locale,
                        "No candidates found in the locations checked. Other directories may exist.",
                        "已检查的位置中未发现候选；其他位置仍可能存在相关目录。")))
                } else {
                    let lines = candidates.into_iter().map(|candidate| {
                        let size = crate::software_review::SoftwareSize::from_evidence(&candidate.size);
                        let size = match size {
                            Some(SoftwareSize::Estimate(value)) => format!("~{}", ui::bytes(value)),
                            Some(SoftwareSize::Measured(value)) => ui::bytes(value),
                            Some(SoftwareSize::AtLeast(value)) => format!("{} {}", copy::pick(locale, "at least", "至少"), ui::bytes(value)),
                            None => copy::pick(locale, "Size unknown", "容量未知").to_string(),
                        };
                        ui::muted(t, format!("{} · {size}", ui::display_path(&candidate.path)))
                    });
                    panel.child(div().id("software-related-paths").flex().flex_col()
                        .max_h(ui::SPACE_6 * 2.0).overflow_y_scroll().children(lines))
                }
            }
        };
        Some(panel.into_any_element())
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
                .gap(ui::SPACE_1)
                .child(ui::card_header(t, copy::pick(locale, "Review", "审阅")))
                .children(lines.iter().map(|line| {
                    div()
                        .text_size(ui::TYPE_BODY)
                        .text_color(rgb(t.text_muted))
                        .child(line.clone())
                })),
        ))
    }

    pub(super) fn optimize_page(&mut self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let board = self.session.optimize_board();
        let phase = board.phase();
        let landing = phase == crate::optimize_board::OptimizePhase::Landing;
        // Landing states the catalogue. Any other phase is named once, as the
        // status text; a failure or an admin wait also has its own banner.
        let bar = if landing {
            let ids: Vec<String> = OptimizeGroup::ALL
                .into_iter()
                .flat_map(crate::optimize_board::ids_in)
                .collect();
            let runnable = ids
                .iter()
                .filter(|id| {
                    devsweep_core::optimize::catalogue_entry(id).is_some_and(|entry| {
                        entry.action_class
                            == devsweep_core::optimize::MaintenanceActionClass::Execute
                    })
                })
                .count();
            page_bar(
                t,
                Some(copy::optimize_catalogue_status(locale, ids.len(), runnable)),
                None,
                Vec::new(),
            )
        } else {
            page_bar(
                t,
                Some(copy::optimize_phase_name(locale, phase.key()).to_string()),
                None,
                Vec::new(),
            )
        };
        let mut page = div()
            .flex()
            .flex_col()
            .gap(ui::SPACE_4)
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
        let mut grid = div().flex().flex_col().gap(ui::SPACE_3);
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
                .flex_col();
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
                    .gap(ui::SPACE_2)
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(ui::SPACE_2)
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
        sheet_frame(bar, page.into_any_element(), None)
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
