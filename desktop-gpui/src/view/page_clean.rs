//! Clean page: landing, scope choice, progress, review, results, and the AI
//! tools subview. Every control calls an existing session action; nothing here
//! scans, previews, or moves files by itself.

use super::*;

/// Disk map canvas size for the designed review.
const MAP_WIDTH: f32 = 640.0;
const MAP_HEIGHT: f32 = 260.0;

impl ShellView {
    pub(super) fn clean_page(&mut self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        if self.session.clean_ai().open() {
            return self.clean_ai_page(t, cx);
        }
        let locale = self.session.locale();
        let phase = self.session.clean_workbench().phase();
        let mut page = div().flex().flex_col().gap(rems(1.25));
        if phase != CleanPhase::Landing {
            let mut actions = Vec::new();
            actions.push(ui::any(
                ui::icon_button(
                    t,
                    "clean-ai-open",
                    "sparkle",
                    copy::clean_ai_open_action(locale),
                    ButtonKind::Ghost,
                )
                .on_click(cx.listener(|this, _, _, cx| this.clean_ai_open(cx))),
            ));
            if matches!(
                phase,
                CleanPhase::Found
                    | CleanPhase::AllClean
                    | CleanPhase::Completed
                    | CleanPhase::Failed
            ) {
                actions.push(ui::any(
                    ui::icon_button(
                        t,
                        "clean-retry",
                        "refresh",
                        copy::clean_retry_action(locale),
                        ButtonKind::Secondary,
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.clean_retry(cx))),
                ));
            }
            page = page.child(self.page_header(
                t,
                "clean",
                copy::mode_name(locale, "clean"),
                copy::mode_subtitle(locale, "clean"),
                actions,
            ));
        }
        page = page.children(self.shared_banners(t));
        page = page.children(self.clean_context_banners(t));
        let body = match phase {
            CleanPhase::Landing => self.clean_landing(t, cx),
            CleanPhase::Scanning | CleanPhase::Cleaning => self.clean_progress(t, cx),
            CleanPhase::Found => self.clean_review(t, cx),
            CleanPhase::AllClean => ui::any(ui::card(t).child(ui::empty_state(
                t,
                "check-circle",
                copy::clean_phase_all_clean(locale),
                copy::pick(
                    locale,
                    "Nothing in the chosen scope needs review right now.",
                    "所选范围内目前没有需要审阅的项目。",
                ),
            ))),
            CleanPhase::Completed => {
                let result = self.clean_result_card(t);
                ui::any(
                    div()
                        .flex()
                        .flex_col()
                        .gap(rems(1.25))
                        .child(result)
                        .when(disk_map_active(&self.session), |d| {
                            d.child(self.clean_review(t, cx))
                        }),
                )
            }
            CleanPhase::Failed => self.clean_failed(t),
        };
        page.child(body).into_any_element()
    }

    /// Root, protection, outcome and failure banners for the Clean workflow.
    fn clean_context_banners(&self, t: &Theme) -> Vec<gpui::AnyElement> {
        let locale = self.session.locale();
        let workbench = self.session.clean_workbench();
        let mut banners = Vec::new();
        if let Some(notice) = self.session.clean_outcome() {
            let tone = match notice {
                CleanOutcomeNotice::TrashReported | CleanOutcomeNotice::Empty => Tone::Success,
                CleanOutcomeNotice::Failed | CleanOutcomeNotice::Unavailable => Tone::Danger,
                CleanOutcomeNotice::Canceled => Tone::Neutral,
                _ => Tone::Warning,
            };
            banners.push(ui::any(ui::banner(
                t,
                tone,
                clean_outcome_line(locale, notice),
            )));
        }
        if workbench.root_session_only() {
            banners.push(ui::any(ui::banner(
                t,
                Tone::Info,
                copy::clean_root_session_only(locale),
            )));
        }
        if self.session.protection_session_only() {
            banners.push(ui::any(ui::banner(
                t,
                Tone::Info,
                copy::clean_protection_session_only(locale),
            )));
        }
        banners
    }

    fn clean_landing(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let workbench = self.session.clean_workbench();
        let prompt = workbench.prompt();
        let k = if prompt == CleanLandingPrompt::None {
            1.0
        } else {
            0.6
        };
        let earth = div()
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .size(rems(15.0 * k))
            .child(
                div()
                    .absolute()
                    .size(rems(15.0 * k))
                    .rounded_full()
                    .bg(t.tint(t.accent, 0x14)),
            )
            .child(
                div()
                    .absolute()
                    .size(rems(12.75 * k))
                    .rounded_full()
                    .border_1()
                    .border_color(t.tint(t.accent, 0x30)),
            )
            .child(
                img(EARTH_NAME)
                    .id("clean-earth")
                    .size(rems(11.25 * k))
                    .with_fallback(|| div().child(SharedString::from("Earth")).into_any_element()),
            );
        let headline = div()
            .flex()
            .flex_col()
            .items_center()
            .gap(rems(0.5))
            .child(
                div()
                    .text_size(rems(1.9))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(copy::mode_subtitle(locale, "clean")),
            )
            .when(prompt == CleanLandingPrompt::None, |d| d.child(
                div()
                    .max_w(rems(34.0))
                    .text_center()
                    .text_size(rems(0.95))
                    .text_color(rgb(t.text_muted))
                    .child(copy::pick(
                        locale,
                        "DevSweep finds build outputs and tool caches, explains what each one costs to rebuild, and moves only what you confirm to the Recycle Bin.",
                        "DevSweep 找出构建产物与工具缓存，说明每一项的重建代价，只把你确认的内容移到回收站。",
                    )),
            ));
        let mut hero = div()
            .flex()
            .flex_col()
            .items_center()
            .gap(rems(1.5))
            .pt(rems(1.0))
            .child(earth)
            .child(headline);
        hero = match prompt {
            CleanLandingPrompt::None => hero.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(rems(0.75))
                    .child(
                        ui::icon_button(
                            t,
                            "clean-scan-primary",
                            "play",
                            copy::clean_scan_primary(locale),
                            ButtonKind::Primary,
                        )
                        .h(rems(2.75))
                        .px(rems(1.5))
                        .text_size(rems(0.95))
                        .on_click(
                            cx.listener(|this, _, window, cx| this.clean_open_scope(window, cx)),
                        ),
                    )
                    .child(
                        ui::icon_button(
                            t,
                            "clean-ai-open",
                            "sparkle",
                            copy::clean_ai_open_action(locale),
                            ButtonKind::Ghost,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.clean_ai_open(cx))),
                    )
                    .when(!self.session.shell().scan_roots().is_empty(), |d| {
                        d.child(self.saved_roots_line(t))
                    }),
            ),
            CleanLandingPrompt::ScopeChoice => hero.child(self.clean_scope_cards(t, cx)),
            CleanLandingPrompt::RootRequired => hero.child(self.clean_root_picker(t, cx)),
            CleanLandingPrompt::ConfirmGlobal => hero.child(self.clean_global_confirm(t, cx)),
        };
        hero.into_any_element()
    }

    fn saved_roots_line(&self, t: &Theme) -> gpui::AnyElement {
        let locale = self.session.locale();
        div()
            .flex()
            .flex_row()
            .flex_wrap()
            .justify_center()
            .items_center()
            .gap(rems(0.4))
            .child(ui::icon("folder", t.text_subtle).size(rems(0.85)))
            .child(
                div()
                    .text_size(rems(0.78))
                    .text_color(rgb(t.text_subtle))
                    .child(copy::project_folders(locale)),
            )
            .children(
                self.session
                    .shell()
                    .scan_roots()
                    .iter()
                    .map(|root| ui::badge(t, root.clone(), Tone::Neutral)),
            )
            .into_any_element()
    }

    fn clean_scope_cards(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let cards = [CleanScope::Project, CleanScope::Global].map(|scope| {
            let (icon, title) = match scope {
                CleanScope::Project => ("code", copy::clean_scope_project(locale)),
                CleanScope::Global => ("globe", copy::clean_scope_global(locale)),
            };
            let hover_border = t.accent;
            div()
                .id(ElementId::Name(SharedString::from(format!(
                    "clean-scope-{}",
                    scope.id()
                ))))
                .role(gpui::Role::Button)
                .aria_label(title)
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .gap(rems(0.6))
                .p(rems(1.25))
                .rounded(rems(0.9))
                .bg(rgb(t.surface))
                .border_1()
                .border_color(rgb(t.border))
                .cursor_pointer()
                .hover(move |s| s.border_color(rgb(hover_border)))
                .keyboard_focus(t)
                .on_click(cx.listener(move |this, _, _, cx| this.clean_scope(scope, cx)))
                .child(ui::icon_tile(t, icon, t.accent))
                .child(
                    div()
                        .text_size(rems(1.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(title),
                )
                .child(ui::muted(t, copy::clean_scope_detail(locale, scope.id())))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(rems(0.3))
                        .text_size(rems(0.8))
                        .text_color(rgb(t.accent))
                        .child(copy::pick(locale, "Choose", "选择"))
                        .child(ui::icon("chevron-right", t.accent).size(rems(0.8))),
                )
        });
        div()
            .w_full()
            .max_w(rems(44.0))
            .flex()
            .flex_col()
            .gap(rems(0.9))
            .child(
                div()
                    .text_center()
                    .text_size(rems(0.95))
                    .font_weight(FontWeight::MEDIUM)
                    .child(copy::clean_scope_prompt(locale)),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .gap(rems(1.0))
                    .children(cards),
            )
            .child(
                div().flex().justify_center().child(
                    ui::button(
                        t,
                        "clean-scope-back",
                        copy::back_label(locale),
                        ButtonKind::Ghost,
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.session.clean_cancel_prompt();
                        cx.notify();
                    })),
                ),
            )
            .into_any_element()
    }

    fn clean_root_picker(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let suggestions = self.session.clean_workbench().suggestions().to_vec();
        let rows = suggestions.into_iter().enumerate().map(|(index, root)| {
            let label = root.display().to_string();
            ui::list_row(
                t,
                ElementId::Name(SharedString::from(format!("clean-root-suggestion-{index}"))),
                false,
                false,
            )
            .role(gpui::Role::Button)
            .aria_label(SharedString::from(label.clone()))
            .on_click(
                cx.listener(move |this, _, _, cx| this.clean_pick_suggestion(root.clone(), cx)),
            )
            .child(ui::icon("folder", t.accent))
            .child(div().flex_1().min_w_0().truncate().child(label))
            .child(ui::icon("chevron-right", t.text_subtle))
        });
        ui::card(t)
            .w_full()
            .max_w(rems(40.0))
            .gap(rems(0.9))
            .child(ui::card_header(t, copy::clean_root_prompt(locale)))
            .child(ui::muted(t, copy::clean_root_suggestions(locale)))
            .child(div().flex().flex_col().gap(rems(0.2)).children(rows))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .justify_end()
                    .gap(rems(0.5))
                    .child(
                        ui::button(
                            t,
                            "clean-root-back",
                            copy::back_label(locale),
                            ButtonKind::Ghost,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.session.clean_cancel_prompt();
                            cx.notify();
                        })),
                    )
                    .child(
                        ui::icon_button(
                            t,
                            "clean-root-choose",
                            "folder",
                            copy::clean_root_choose(locale),
                            ButtonKind::Primary,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.clean_pick_root(cx))),
                    ),
            )
            .into_any_element()
    }

    fn clean_global_confirm(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        ui::card(t)
            .w_full()
            .max_w(rems(40.0))
            .gap(rems(0.9))
            .child(ui::card_header(t, copy::clean_scope_global(locale)))
            .child(ui::banner(
                t,
                Tone::Info,
                copy::clean_global_confirm(locale),
            ))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .justify_end()
                    .gap(rems(0.5))
                    .child(
                        ui::button(
                            t,
                            "clean-global-back",
                            copy::back_label(locale),
                            ButtonKind::Ghost,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.session.clean_cancel_prompt();
                            cx.notify();
                        })),
                    )
                    .child(
                        ui::icon_button(
                            t,
                            "clean-global-confirm",
                            "play",
                            copy::clean_global_scan_action(locale),
                            ButtonKind::Primary,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.clean_confirm_global(cx))),
                    ),
            )
            .into_any_element()
    }

    fn clean_progress(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let workbench = self.session.clean_workbench();
        let phase = workbench.phase();
        let (title, icon) = if phase == CleanPhase::Scanning {
            (
                copy::clean_phase_scanning(locale, workbench.found_so_far()),
                "search",
            )
        } else {
            (copy::clean_phase_cleaning(locale).to_string(), "trash")
        };
        ui::card(t)
            .id("clean-progress")
            .role(gpui::Role::Status)
            .aria_label(SharedString::from(title.clone()))
            .gap(rems(1.0))
            .p(rems(1.75))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(rems(0.9))
                    .child(ui::icon_tile(t, icon, t.accent))
                    .child(
                        div()
                            .flex_1()
                            .text_size(rems(1.05))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title),
                    )
                    .child(
                        ui::icon_button(
                            t,
                            ElementId::Name(SharedString::from(format!(
                                "clean-cancel-{}",
                                phase.id()
                            ))),
                            "stop",
                            copy::clean_cancel_action(locale),
                            ButtonKind::Secondary,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.clean_cancel(cx))),
                    ),
            )
            .child(ui::progress_track(t))
            .child(ui::muted(
                t,
                copy::pick(
                    locale,
                    "Read-only discovery. Nothing moves until you preview and confirm a selection.",
                    "只读发现。在你预览并确认选择之前，不会移动任何内容。",
                ),
            ))
            .into_any_element()
    }

    fn clean_failed(&self, t: &Theme) -> gpui::AnyElement {
        let locale = self.session.locale();
        let detail = self
            .session
            .clean_workbench()
            .failure_detail()
            .map(str::to_string)
            .unwrap_or_default();
        ui::card(t)
            .child(ui::empty_state(
                t,
                "alert",
                copy::clean_phase_failed(locale),
                detail,
            ))
            .into_any_element()
    }

    fn clean_result_card(&self, t: &Theme) -> gpui::AnyElement {
        let locale = self.session.locale();
        let workbench = self.session.clean_workbench();
        let (moved, total) = workbench.result_counts().unwrap_or((0, 0));
        let tone = if moved == total { t.success } else { t.warning };
        ui::card(t)
            .flex_row()
            .items_center()
            .gap(rems(1.0))
            .child(ui::icon_tile(t, "check-circle", tone))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .gap(rems(0.15))
                    .child(
                        div()
                            .text_size(rems(1.05))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(copy::clean_phase_completed(locale)),
                    )
                    .when_some(workbench.result_counts(), |d, (moved, total)| {
                        d.child(ui::muted(
                            t,
                            copy::clean_result_counts(locale, moved, total),
                        ))
                    }),
            )
            .into_any_element()
    }

    /// Found review: stat tiles, disk map, row list, and the staged actions.
    fn clean_review(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let summary = clean_map::summarize(&map_inputs(&self.session));
        let stats = div()
            .id("clean-map-stats")
            .flex()
            .flex_row()
            .flex_wrap()
            .gap(rems(0.75))
            .child(ui::stat(
                t,
                copy::clean_map_card_mapped(locale),
                copy::clean_bytes_short(summary.mapped_bytes),
                copy::clean_map_count(locale, summary.mapped_count),
                t.accent,
            ))
            .child(ui::stat(
                t,
                copy::clean_map_card_recommended(locale),
                copy::clean_bytes_short(summary.recommended_bytes),
                copy::clean_map_count(locale, summary.recommended_count),
                t.success,
            ))
            .child(ui::stat(
                t,
                copy::clean_map_card_review(locale),
                copy::clean_bytes_short(summary.review_bytes),
                copy::clean_map_count(locale, summary.review_count),
                t.warning,
            ))
            .child(ui::stat(
                t,
                copy::clean_map_card_blocked(locale),
                copy::clean_bytes_short(summary.blocked_bytes),
                copy::clean_map_count(locale, summary.blocked_count),
                t.text_subtle,
            ));
        div()
            .flex()
            .flex_col()
            .gap(rems(1.25))
            .child(stats)
            .when(disk_map_active(&self.session), |d| {
                d.child(self.clean_disk_map(t, cx))
            })
            .child(self.clean_rows_card(t, cx))
            .into_any_element()
    }

    fn clean_disk_map(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let map = clean_map::project_disk_map(
            &map_inputs(&self.session),
            MapBounds {
                width: MAP_WIDTH,
                height: MAP_HEIGHT,
            },
        );
        let workbench = self.session.clean_workbench();
        let mut canvas = div()
            .id("clean-disk-map-canvas")
            .relative()
            .flex_shrink_0()
            .w(px(MAP_WIDTH))
            .h(px(MAP_HEIGHT))
            .overflow_hidden()
            .rounded(rems(0.6))
            .bg(rgb(t.surface_alt));
        if map.groups.is_empty() {
            canvas = canvas.child(
                div()
                    .p(rems(1.0))
                    .child(ui::muted(t, copy::clean_map_empty(locale))),
            );
        }
        for (group_index, group) in map.groups.iter().enumerate() {
            let title = if group.other {
                copy::clean_map_other(locale, group.represented)
            } else {
                group.title.clone()
            };
            let group_label = format!(
                "{} {} {}",
                title,
                copy::clean_bytes_short(group.bytes),
                copy::clean_map_category(locale, group.category.as_str())
            );
            let mut node = div()
                .id(ElementId::Name(SharedString::from(format!(
                    "clean-map-group-{group_index}"
                ))))
                .absolute()
                .left(px(group.x + 1.0))
                .top(px(group.y + 1.0))
                .w(px((group.width - 2.0).max(1.0)))
                .h(px((group.height - 2.0).max(1.0)))
                .rounded(px(6.0))
                .overflow_hidden()
                .aria_label(SharedString::from(group_label));
            if group.height >= 30.0 && group.width >= 60.0 {
                node = node.child(
                    div()
                        .absolute()
                        .left(px(6.0))
                        .top(px(2.0))
                        .right(px(4.0))
                        .truncate()
                        .text_size(px(10.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(t.text_muted))
                        .child(SharedString::from(format!(
                            "{title} · {}",
                            copy::clean_bytes_short(group.bytes)
                        ))),
                );
            }
            for (leaf_index, leaf) in group.leaves.iter().enumerate() {
                let hue = t.category(category_index(leaf.category));
                let selected = leaf
                    .id
                    .as_deref()
                    .is_some_and(|id| workbench.is_selected(id));
                let leaf_title = if leaf.other {
                    copy::clean_map_other(locale, leaf.represented)
                } else {
                    leaf.title.clone()
                };
                let label = format!(
                    "{} {} {} {} {}",
                    copy::clean_map_category(locale, leaf.category.as_str()),
                    leaf_title,
                    copy::clean_bytes_short(leaf.bytes),
                    copy::clean_map_band(locale, leaf.band.as_str()),
                    if selected {
                        copy::clean_row_selected_marker(locale)
                    } else {
                        copy::clean_row_unselected_marker(locale)
                    }
                );
                let element_id = match leaf.id.as_deref() {
                    Some(id) => format!("clean-map-tile-{id}"),
                    None => format!("clean-map-other-{group_index}-{leaf_index}"),
                };
                let (x, y) = (leaf.x + 1.0, leaf.y + 1.0);
                let (w, h) = ((leaf.width - 2.0).max(1.0), (leaf.height - 2.0).max(1.0));
                let mut tile = div()
                    .id(ElementId::Name(SharedString::from(element_id)))
                    .absolute()
                    .left(px(x))
                    .top(px(y))
                    .w(px(w))
                    .h(px(h))
                    .overflow_hidden()
                    .rounded(px(5.0))
                    .px(px(6.0))
                    .py(px(4.0))
                    .bg(t.tint(hue, if selected { 0x88 } else { 0x38 }))
                    .border_1()
                    .border_color(if selected {
                        rgb(t.text)
                    } else {
                        t.tint(hue, 0x90)
                    })
                    .aria_label(SharedString::from(label));
                if let Some(id) = leaf.id.clone() {
                    tile = tile
                        .role(gpui::Role::Button)
                        .cursor_pointer()
                        .hover(|s| s.opacity(0.85))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.session.clean_click_row(&id);
                            cx.notify();
                        }));
                }
                if w >= 56.0 && h >= 24.0 {
                    tile = tile.child(
                        div()
                            .truncate()
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(t.text))
                            .child(SharedString::from(leaf_title)),
                    );
                    if h >= 40.0 {
                        tile = tile.child(
                            div()
                                .text_size(px(10.5))
                                .text_color(rgb(t.text_muted))
                                .child(SharedString::from(copy::clean_bytes_short(leaf.bytes))),
                        );
                    }
                }
                node = node.child(tile);
            }
            canvas = canvas.child(node);
        }
        let legend = div()
            .flex()
            .flex_col()
            .gap(rems(0.55))
            .min_w(rems(10.0))
            .child(ui::eyebrow(
                t,
                copy::pick(locale, "CONTENT CLASS", "内容类别"),
            ))
            .children(map.categories.iter().map(|category| {
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(rems(0.5))
                    .child(
                        div()
                            .size(rems(0.7))
                            .rounded(rems(0.2))
                            .bg(rgb(t.category(category_index(*category)))),
                    )
                    .child(
                        div()
                            .text_size(rems(0.8))
                            .text_color(rgb(t.text_muted))
                            .child(copy::clean_map_category(locale, category.as_str())),
                    )
            }));
        ui::card(t)
            .id("clean-disk-map")
            .role(gpui::Role::Group)
            .aria_label(copy::clean_map_title(locale))
            .gap(rems(1.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(rems(0.2))
                    .child(ui::card_header(t, copy::clean_map_title(locale)))
                    .child(ui::muted(t, copy::clean_map_subtitle(locale)))
                    .when(
                        !self.session.shell().row_query(Mode::Clean).is_empty(),
                        |d| d.child(ui::muted(t, copy::clean_map_filtered_note(locale))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_start()
                    .gap(rems(1.5))
                    .child(canvas)
                    .child(legend),
            )
            .into_any_element()
    }

    fn clean_rows_card(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let shell = self.session.shell();
        let workbench = self.session.clean_workbench();
        let found = workbench.phase() == CleanPhase::Found;
        let sort_label = match workbench.sort() {
            CleanSort::ReportOrder => copy::clean_sort_report_order(locale),
            CleanSort::Name => copy::clean_sort_name(locale),
            CleanSort::Size => copy::clean_sort_size(locale),
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
            .when(found, |bar| {
                bar.child(
                    ui::icon_button(t, "clean-sort", "sort", sort_label, ButtonKind::Ghost)
                        .on_click(cx.listener(|this, _, _, cx| this.clean_sort(cx))),
                )
                .child(
                    ui::button(
                        t,
                        "clean-select-recommended",
                        copy::clean_select_recommended(locale),
                        ButtonKind::Secondary,
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.clean_select_recommended(cx))),
                )
                .child(
                    ui::button(
                        t,
                        "clean-clear-selection",
                        copy::clean_clear_selection(locale),
                        ButtonKind::Ghost,
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.clean_clear_selection(cx))),
                )
            });
        let cursor = shell.selection(Mode::Clean).map(str::to_string);
        let mut rows = div()
            .id("clean-rows")
            .role(gpui::Role::List)
            .flex()
            .flex_col()
            .gap(rems(0.15));
        for id in shell.visible_row_ids(Mode::Clean) {
            let Some(row) = workbench.row(id) else {
                continue;
            };
            let focused = cursor.as_deref() == Some(id.as_str());
            rows = rows.child(self.clean_row(t, row, focused, cx));
        }
        let total = shell.filtered_row_ids(Mode::Clean).len();
        let start = shell.row_window_start(Mode::Clean);
        let shown = shell.visible_row_ids(Mode::Clean).len();
        let pager = (total > shown).then(|| {
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .pt(rems(0.5))
                .child(ui::muted(
                    t,
                    format!(
                        "{}–{} / {total}",
                        (start + 1).min(total),
                        (start + shown).min(total)
                    ),
                ))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(rems(0.4))
                        .child(
                            ui::icon_button(
                                t,
                                "clean-page-up",
                                "chevron-left",
                                copy::pick(locale, "Previous", "上一页"),
                                ButtonKind::Ghost,
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.session.clean_scroll_rows(
                                    -(crate::shell::VISIBLE_ROW_CAPACITY as isize),
                                );
                                cx.notify();
                            })),
                        )
                        .child(
                            ui::icon_button(
                                t,
                                "clean-page-down",
                                "chevron-right",
                                copy::pick(locale, "Next", "下一页"),
                                ButtonKind::Ghost,
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.session
                                    .clean_scroll_rows(crate::shell::VISIBLE_ROW_CAPACITY as isize);
                                cx.notify();
                            })),
                        ),
                )
        });
        ui::card(t)
            .id("clean-map-recommendations")
            .role(gpui::Role::Group)
            .aria_label(copy::clean_map_recommendations(locale))
            .gap(rems(0.9))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(rems(0.2))
                    .child(ui::card_header(t, copy::clean_map_recommendations(locale)))
                    .child(ui::muted(t, copy::clean_map_recommendations_hint(locale))),
            )
            .child(toolbar)
            .child(ui::divider(t))
            .child(rows)
            .children(pager)
            .when(found, |c| c.child(self.clean_action_bar(t, cx)))
            .into_any_element()
    }

    fn clean_row(
        &self,
        t: &Theme,
        row: &CleanRow,
        focused: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let locale = self.session.locale();
        let workbench = self.session.clean_workbench();
        let selected = workbench.is_selected(&row.id);
        let protected = row_protected(&self.session, row);
        let ignored = workbench.is_ignored(&row.id);
        let selectable = workbench.row_selectable(row);
        let text =
            crate::clean_workbench::row_text(locale, row, selected, focused, protected, ignored);
        let category = clean_map::category_of(&row.rule_id);
        let band = clean_map::review_band(selectable, row.selected_by_default);
        let band_tone = match band {
            ReviewBand::Recommended => Tone::Success,
            ReviewBand::NeedsReview => Tone::Warning,
            ReviewBand::NotSelectable => Tone::Neutral,
        };
        let title = clean_map::leaf_title(&row.rule_id, row.path.as_deref());
        let subtitle = match row.path.as_deref() {
            Some(path) => path.to_string(),
            None => copy::clean_row_global(locale).to_string(),
        };
        let element_id = SharedString::from(format!("clean-row-{}", row.id));
        let clicked = row.id.clone();
        let mut badges = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap(rems(0.3))
            .child(ui::badge(
                t,
                copy::clean_map_band(locale, band.as_str()),
                band_tone,
            ));
        if !row.size_complete {
            badges = badges.child(ui::badge(t, row.confidence(locale), Tone::Warning));
        }
        if row.inspect_only {
            badges = badges.child(ui::badge(
                t,
                copy::clean_row_inspect_only(locale),
                Tone::Neutral,
            ));
        }
        if protected {
            badges = badges.child(ui::badge(t, copy::clean_row_protected(locale), Tone::Info));
        }
        if ignored {
            badges = badges.child(ui::badge(t, copy::clean_row_ignored(locale), Tone::Neutral));
        }
        ui::list_row(t, ElementId::Name(element_id.clone()), selected, focused)
            .accessibility_id(element_id)
            .aria_label(SharedString::from(text))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.session.clean_click_row(&clicked);
                cx.notify();
            }))
            .child(ui::checkbox(t, selected, selectable))
            .child(
                div()
                    .w(rems(0.25))
                    .h(rems(2.0))
                    .flex_shrink_0()
                    .rounded_full()
                    .bg(rgb(t.category(category_index(category)))),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(rems(0.25))
                    .child(ui::two_line(t, title, subtitle))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .flex_wrap()
                            .items_center()
                            .gap(rems(0.5))
                            .child(badges)
                            .child(
                                div()
                                    .text_size(rems(0.72))
                                    .text_color(rgb(t.text_subtle))
                                    .child(copy::clean_row_rebuild_cost(locale, &row.rule_id)),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(rems(0.95))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(copy::clean_bytes_short(row.bytes)),
            )
            .into_any_element()
    }

    /// Selection summary and the staged preview/confirm actions.
    fn clean_action_bar(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let workbench = self.session.clean_workbench();
        let selected_bytes = workbench.selected_ids().iter().fold(0u64, |total, id| {
            total.saturating_add(workbench.row(id).map(|row| row.bytes).unwrap_or(0))
        });
        let ready = workbench.preview_matches_current();
        let mut focused_actions = Vec::new();
        if let Some(cursor) = self.session.clean_focused_row_id() {
            let protected = workbench.is_protected(&cursor);
            let protect_id = cursor.clone();
            focused_actions.push(ui::any(
                ui::icon_button(
                    t,
                    "clean-protect-row",
                    "shield",
                    if protected {
                        copy::clean_unprotect_action(locale)
                    } else {
                        copy::clean_protect_action(locale)
                    },
                    ButtonKind::Ghost,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.session.clean_toggle_row_protection(&protect_id);
                    cx.notify();
                })),
            ));
            let ignored = workbench.is_ignored(&cursor);
            let ignore_id = cursor.clone();
            focused_actions.push(ui::any(
                ui::icon_button(
                    t,
                    "clean-ignore-row",
                    "eye",
                    if ignored {
                        copy::clean_unignore_action(locale)
                    } else {
                        copy::clean_ignore_action(locale)
                    },
                    ButtonKind::Ghost,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.session.clean_toggle_row_ignored(&ignore_id);
                    cx.notify();
                })),
            ));
        }
        let selection =
            copy::clean_selection_count(locale, workbench.selected_count(), workbench.rows().len());
        div()
            .id("clean-actions")
            .flex()
            .flex_col()
            .gap(rems(0.6))
            .mt(rems(0.25))
            .p(rems(0.9))
            .rounded(rems(0.7))
            .bg(rgb(t.surface_alt))
            .border_1()
            .border_color(rgb(t.border))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(rems(0.75))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w(rems(12.0))
                            .child(
                                div()
                                    .text_size(rems(1.1))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(copy::clean_bytes_short(selected_bytes)),
                            )
                            .child(ui::muted(t, selection.clone())),
                    )
                    .children(focused_actions)
                    .child(
                        ui::icon_button(
                            t,
                            "clean-preview",
                            "eye",
                            copy::clean_preview_action(locale),
                            ButtonKind::Secondary,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.clean_preview(cx))),
                    )
                    .child(
                        ui::icon_button(
                            t,
                            "clean-confirm",
                            "trash",
                            format!("{} · {selection}", copy::clean_confirm_action(locale)),
                            if ready {
                                ButtonKind::Primary
                            } else {
                                ButtonKind::Secondary
                            },
                        )
                        .when(!ready, |b| b.opacity(0.6))
                        .on_click(cx.listener(|this, _, _, cx| this.clean_confirm_execute(cx))),
                    ),
            )
            .when_some(workbench.preview(), |bar, preview| {
                bar.child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(rems(0.5))
                        .child(ui::icon(
                            if ready { "check-circle" } else { "alert" },
                            if ready { t.success } else { t.warning },
                        ))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(rems(0.76))
                                .text_color(rgb(t.text_muted))
                                .child(copy::clean_preview_digest_line(
                                    locale,
                                    &preview.digest,
                                    preview.count,
                                )),
                        ),
                )
            })
            .into_any_element()
    }

    /// AI tools subview. It never scans until its own scan button is pressed.
    fn clean_ai_page(&mut self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let view = self.session.clean_ai();
        let phase = view.phase();
        let busy = matches!(phase, AiSubviewPhase::Scanning | AiSubviewPhase::Cleaning);
        let mut actions = Vec::new();
        if !busy {
            actions.push(ui::any(
                ui::icon_button(
                    t,
                    "clean-ai-provider",
                    "refresh",
                    copy::clean_ai_provider(locale, view.provider().as_str()),
                    ButtonKind::Secondary,
                )
                .on_click(cx.listener(|this, _, _, cx| this.clean_ai_provider(cx))),
            ));
        }
        actions.push(ui::any(
            ui::icon_button(
                t,
                "clean-ai-close",
                "chevron-left",
                copy::clean_ai_close_action(locale),
                ButtonKind::Ghost,
            )
            .on_click(cx.listener(|this, _, _, cx| this.clean_ai_close(cx))),
        ));
        let mut page = div()
            .id("clean-ai-controls")
            .role(gpui::Role::Group)
            .aria_label(copy::clean_ai_title(locale))
            .flex()
            .flex_col()
            .gap(rems(1.25))
            .child(self.page_header(
                t,
                "sparkle",
                copy::clean_ai_title(locale),
                copy::clean_ai_subtitle(locale),
                actions,
            ))
            .children(self.shared_banners(t));
        if let Some(notice) = view.notice() {
            page = page.child(ui::banner(
                t,
                Tone::Warning,
                clean_outcome_line(locale, notice),
            ));
        }
        if let Some(home) = view.home() {
            page = page.child(ui::muted(
                t,
                copy::clean_ai_home_line(locale, &home.display().to_string()),
            ));
        }
        let body = match phase {
            AiSubviewPhase::Landing => ui::any(
                ui::card(t)
                    .items_center()
                    .child(ui::empty_state(
                        t,
                        "sparkle",
                        copy::clean_ai_title(locale),
                        copy::clean_ai_landing_prompt(locale),
                    ))
                    .child(
                        ui::icon_button(
                            t,
                            "clean-ai-scan",
                            "search",
                            copy::clean_ai_scan_action(locale),
                            ButtonKind::Primary,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.clean_ai_scan(cx))),
                    ),
            ),
            AiSubviewPhase::Scanning | AiSubviewPhase::Cleaning => {
                let title = if phase == AiSubviewPhase::Scanning {
                    copy::clean_ai_phase_scanning(locale)
                } else {
                    copy::clean_ai_phase_cleaning(locale)
                };
                ui::any(
                    ui::card(t)
                        .gap(rems(1.0))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(rems(0.9))
                                .child(ui::icon_tile(t, "search", t.accent))
                                .child(
                                    div()
                                        .flex_1()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(title),
                                )
                                .child(
                                    ui::icon_button(
                                        t,
                                        ElementId::Name(SharedString::from(format!(
                                            "clean-ai-cancel-{}",
                                            phase.id()
                                        ))),
                                        "stop",
                                        copy::clean_cancel_action(locale),
                                        ButtonKind::Secondary,
                                    )
                                    .on_click(cx.listener(|this, _, _, cx| this.clean_cancel(cx))),
                                ),
                        )
                        .child(ui::progress_track(t)),
                )
            }
            AiSubviewPhase::Failed | AiSubviewPhase::Completed => {
                let (icon, title, tone) = if phase == AiSubviewPhase::Failed {
                    ("alert", copy::clean_ai_phase_failed(locale), t.danger)
                } else {
                    (
                        "check-circle",
                        copy::clean_ai_phase_completed(locale),
                        t.success,
                    )
                };
                ui::any(
                    ui::card(t)
                        .flex_row()
                        .items_center()
                        .gap(rems(1.0))
                        .child(ui::icon_tile(t, icon, tone))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
                                .when_some(view.failure_detail(), |d, detail| {
                                    d.child(ui::muted(t, detail.to_string()))
                                }),
                        )
                        .child(
                            ui::icon_button(
                                t,
                                "clean-ai-scan",
                                "refresh",
                                copy::clean_ai_scan_action(locale),
                                ButtonKind::Secondary,
                            )
                            .on_click(cx.listener(|this, _, _, cx| this.clean_ai_scan(cx))),
                        ),
                )
            }
            AiSubviewPhase::Review => self.clean_ai_review(t, cx),
        };
        page = page.child(body);
        if view.home().is_some() && !busy {
            page = page.child(self.clean_ai_recovery(t, cx));
        }
        page.into_any_element()
    }

    fn clean_ai_review(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let view = self.session.clean_ai();
        let segments = AiCategory::ALL.map(|category| {
            let label = format!(
                "{} {}",
                copy::clean_ai_category_name(locale, category.id()),
                view.category_count(category)
            );
            ui::segment(
                t,
                ElementId::Name(SharedString::from(format!(
                    "clean-ai-category-{}",
                    category.id()
                ))),
                label,
                view.category() == category,
            )
            .on_click(cx.listener(move |this, _, _, cx| this.clean_ai_category(category, cx)))
        });
        let cursor = self
            .session
            .shell()
            .selection(Mode::Clean)
            .map(str::to_string);
        let mut rows = div()
            .id("clean-ai-rows")
            .role(gpui::Role::List)
            .flex()
            .flex_col()
            .gap(rems(0.15));
        for id in self.session.shell().visible_row_ids(Mode::Clean) {
            let Some(row) = view.row(id) else {
                continue;
            };
            let focused = cursor.as_deref() == Some(id.as_str());
            let selected = view.is_selected(id);
            let text = view.row_text(locale, id, focused).unwrap_or_default();
            let (status, tone) = if row.sensitive {
                (copy::clean_ai_row_sensitive(locale), Tone::Danger)
            } else if row.selectable() {
                (copy::clean_ai_row_selectable(locale), Tone::Success)
            } else {
                (copy::clean_ai_row_inspect_only(locale), Tone::Neutral)
            };
            let element_id = SharedString::from(format!("ai-row-{id}"));
            let clicked = id.clone();
            rows = rows.child(
                ui::list_row(t, ElementId::Name(element_id.clone()), selected, focused)
                    .accessibility_id(element_id)
                    .aria_label(SharedString::from(text))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.session.clean_click_row(&clicked);
                        cx.notify();
                    }))
                    .child(ui::checkbox(t, selected, row.selectable()))
                    .child(ui::two_line(
                        t,
                        row.relative_path.clone(),
                        copy::clean_ai_row_class(locale, row.class.as_str()),
                    ))
                    .child(ui::badge(t, status, tone)),
            );
        }
        if view.active_row_ids().is_empty() {
            rows = rows.child(ui::empty_state(
                t,
                "check-circle",
                copy::clean_ai_active_category(
                    locale,
                    copy::clean_ai_category_name(locale, view.category().id()),
                    0,
                ),
                String::new(),
            ));
        }
        let ready = view.preview_matches_current();
        let selection =
            copy::clean_ai_selection_count(locale, view.selected_count(), view.selectable_count());
        ui::card(t)
            .gap(rems(0.9))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(rems(0.75))
                    .child(
                        ui::segmented(t)
                            .id("clean-ai-categories")
                            .role(gpui::Role::TabList)
                            .children(segments),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(rems(0.5))
                            .w(rems(16.0))
                            .child(ui::icon("search", t.text_subtle))
                            .child(div().flex_1().child(self.search.clone())),
                    ),
            )
            .child(ui::muted(
                t,
                copy::clean_ai_phase_review(locale, view.rows().len(), view.sensitive_count()),
            ))
            .when(view.hidden_tool_rows() > 0, |c| {
                c.child(ui::muted(
                    t,
                    copy::clean_ai_tool_rows(locale, view.hidden_tool_rows()),
                ))
            })
            .child(ui::divider(t))
            .child(rows)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(rems(0.6))
                    .p(rems(0.9))
                    .rounded(rems(0.7))
                    .bg(rgb(t.surface_alt))
                    .border_1()
                    .border_color(rgb(t.border))
                    .child(div().flex_1().child(ui::muted(t, selection.clone())))
                    .child(
                        ui::button(
                            t,
                            "clean-ai-clear-selection",
                            copy::clean_clear_selection(locale),
                            ButtonKind::Ghost,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.clean_ai_clear_selection(cx))),
                    )
                    .child(
                        ui::icon_button(
                            t,
                            "clean-ai-preview",
                            "eye",
                            copy::clean_preview_action(locale),
                            ButtonKind::Secondary,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.clean_ai_preview(cx))),
                    )
                    .child(
                        ui::icon_button(
                            t,
                            "clean-ai-confirm",
                            "trash",
                            format!("{} · {selection}", copy::clean_ai_confirm_action(locale)),
                            if ready {
                                ButtonKind::Primary
                            } else {
                                ButtonKind::Secondary
                            },
                        )
                        .when(!ready, |b| b.opacity(0.6))
                        .on_click(cx.listener(|this, _, _, cx| this.clean_ai_confirm_execute(cx))),
                    ),
            )
            .when_some(view.preview(), |c, preview| {
                c.child(ui::muted(
                    t,
                    copy::clean_ai_digest_line(locale, &preview.digest, preview.count),
                ))
            })
            .into_any_element()
    }

    fn clean_ai_recovery(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let view = self.session.clean_ai();
        let mut list = div().flex().flex_col().gap(rems(0.3));
        if view.recovery_loaded() && view.recovery().is_empty() {
            list = list.child(ui::muted(t, copy::clean_ai_recovery_empty(locale)));
        }
        for bundle in view.recovery() {
            let bundle_id = bundle.id.clone();
            list = list.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(rems(0.6))
                    .px(rems(0.75))
                    .py(rems(0.5))
                    .rounded(rems(0.5))
                    .bg(rgb(t.surface_alt))
                    .child(ui::icon("history", t.text_muted))
                    .child(ui::two_line(
                        t,
                        bundle.relative_path.clone(),
                        bundle.id.clone(),
                    ))
                    .child(
                        ui::button(
                            t,
                            ElementId::Name(SharedString::from(format!(
                                "clean-ai-restore-{bundle_id}"
                            ))),
                            copy::clean_ai_restore_action(locale),
                            ButtonKind::Secondary,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.clean_ai_restore(bundle_id.clone(), cx)
                        })),
                    ),
            );
        }
        if let Some(plan) = view.planned_restore()
            && let Some(target) = plan.targets.first()
        {
            list = list.child(ui::banner(
                t,
                Tone::Info,
                copy::clean_ai_restore_ready(locale, &target.relative_path, &target.content_sha256),
            ));
        }
        if let Some(bundle) = view.restore_refused_bundle() {
            list = list.child(ui::banner(
                t,
                Tone::Warning,
                format!("{} {bundle}", copy::clean_ai_restore_refused(locale)),
            ));
        }
        ui::card(t)
            .gap(rems(0.75))
            .child(
                ui::card_header(t, copy::clean_ai_recovery_title(locale)).child(
                    ui::icon_button(
                        t,
                        "clean-ai-recovery-refresh",
                        "refresh",
                        copy::clean_ai_recovery_refresh(locale),
                        ButtonKind::Ghost,
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.clean_ai_refresh_recovery(cx))),
                ),
            )
            .child(list)
            .into_any_element()
    }
}

fn category_index(category: clean_map::MapCategory) -> usize {
    use clean_map::MapCategory;
    match category {
        MapCategory::BuildOutput => 0,
        MapCategory::SafeToClear => 1,
        MapCategory::AgentHistory => 2,
        MapCategory::AiModels => 3,
        MapCategory::Docker => 4,
        MapCategory::SystemApps => 5,
        MapCategory::Other => 6,
    }
}
