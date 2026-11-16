//! Clean page: landing, scope choice, progress, review, results, and the AI
//! tools subview. Every control calls an existing session action; nothing here
//! scans, previews, or moves files by itself.

use super::*;

/// Disk map canvas size for the designed review.
const MAP_WIDTH: f32 = 640.0;
const MAP_HEIGHT: f32 = 260.0;

fn preview_status(
    t: &Theme,
    locale: copy::Locale,
    id: &'static str,
    ready: bool,
    digest_line: String,
) -> gpui::Stateful<gpui::Div> {
    let (tone, label) = if ready {
        (Tone::Success, digest_line)
    } else {
        (
            Tone::Warning,
            copy::clean_outcome_stale_preview(locale).to_owned(),
        )
    };
    ui::banner(t, tone, label.clone())
        .id(id)
        .role(gpui::Role::Status)
        .aria_label(label)
}

impl ShellView {
    pub(super) fn clean_page(
        &mut self,
        t: &Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        if self.session.clean_ai().open() {
            return self.clean_ai_page(t, cx);
        }
        let locale = self.session.locale();
        let phase = self.session.clean_workbench().phase();
        // Reserve chrome, scope, toolbar and actions; the row viewport uses the
        // remaining height. Short lists stay compact and long lists scroll.
        let row_height =
            (window.viewport_size().height - window.rem_size() * 26.0).max(window.rem_size() * 8.0);
        let mut page = div().flex().flex_col().gap(ui::SPACE_4);
        let mut actions = vec![ui::any(
            ui::icon_button(
                t,
                "clean-ai-open",
                "sparkle",
                copy::clean_ai_open_action(locale),
                ButtonKind::Ghost,
            )
            .on_click(cx.listener(|this, _, _, cx| this.clean_ai_open(cx))),
        )];
        if phase == CleanPhase::Landing
            && self.session.clean_workbench().prompt() == CleanLandingPrompt::None
        {
            actions.push(ui::any(
                ui::button(
                    t,
                    "clean-header-scan",
                    copy::pick(locale, "Scan", "扫描"),
                    ButtonKind::Primary,
                )
                .on_click(cx.listener(|this, _, _, cx| this.clean_start_scope(cx))),
            ));
        }
        if matches!(
            phase,
            CleanPhase::Found | CleanPhase::AllClean | CleanPhase::Completed | CleanPhase::Failed
        ) && !(phase == CleanPhase::Found && self.session.clean_workbench().reviewing())
        {
            if self.session.clean_workbench().selected_scope() != CleanScope::Global {
                actions.push(ui::any(
                    ui::icon_button(
                        t,
                        "clean-root-choose",
                        "folder",
                        copy::pick(locale, "Change folder", "更换文件夹"),
                        ButtonKind::Ghost,
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.clean_pick_root(cx)))
                    .on_a11y_action(
                        gpui::AccessibleAction::Click,
                        on_view_a11y(cx, |this, cx| this.clean_pick_root(cx)),
                    ),
                ));
            }
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
        if phase == CleanPhase::Found && !self.session.clean_workbench().reviewing() {
            actions.push(ui::any(
                ui::button(
                    t,
                    "clean-review-results",
                    copy::clean_review_action(locale),
                    ButtonKind::Primary,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.session.clean_begin_review();
                    cx.notify();
                })),
            ));
        }
        if phase == CleanPhase::Found && self.session.clean_workbench().reviewing() {
            let workbench = self.session.clean_workbench();
            let previewed = workbench.preview_matches_current()
                || workbench.native_fixture() == Some("previewed");
            let enabled = if previewed {
                workbench.preview_matches_current()
            } else {
                workbench.selected_count() > 0 && workbench.native_fixture().is_none()
            };
            actions.push(ui::any(
                ui::button(
                    t,
                    if previewed {
                        "clean-header-confirm"
                    } else {
                        "clean-header-preview"
                    },
                    if previewed {
                        copy::clean_confirm_action(locale)
                    } else {
                        copy::clean_preview_action(locale)
                    },
                    if !enabled {
                        ButtonKind::Disabled
                    } else if previewed {
                        ButtonKind::Danger
                    } else {
                        ButtonKind::Primary
                    },
                )
                .when(enabled, |button| {
                    button.on_click(cx.listener(move |this, _, _, cx| {
                        if previewed {
                            this.clean_confirm_execute(cx);
                        } else {
                            this.clean_preview(cx);
                        }
                    }))
                }),
            ));
        }
        let figure = None;
        // Landing has no phase sentence of its own, so the first row states
        // the one fact there is: whether a folder is chosen and that nothing
        // has been scanned.
        let status = clean_phase_line(&self.session).or_else(|| {
            (phase == CleanPhase::Landing).then(|| {
                let project = self.session.clean_workbench().selected_scope() != CleanScope::Global;
                copy::clean_landing_status(locale, project, self.session.shell().scan_roots().len())
            })
        });
        let status = status.map(|status| {
            if self.session.clean_workbench().global_omitted() {
                format!("{status} · {}", copy::clean_global_omitted(locale))
            } else {
                status
            }
        });
        let bar = page_bar(t, status, figure, actions);
        let footer = (phase == CleanPhase::Found && self.session.clean_workbench().reviewing())
            .then(|| self.clean_action_bar(t, cx));
        if !matches!(phase, CleanPhase::Scanning | CleanPhase::Cleaning) {
            page = page.child(self.clean_scope_tabs(t, cx));
        }
        page = page.children(self.shared_banners(t));
        page = page.children(self.clean_context_banners(t));
        let body = match phase {
            CleanPhase::Landing => self.clean_landing(t, cx),
            CleanPhase::Scanning | CleanPhase::Cleaning => self.clean_progress(t, cx),
            CleanPhase::Found => self.clean_review(t, row_height, cx),
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
            CleanPhase::Completed => self.clean_result_card(t, cx),
            CleanPhase::Failed if self.session.clean_workbench().execution().is_some() => {
                self.clean_result_card(t, cx)
            }
            CleanPhase::Failed => self.clean_failed(t),
        };
        sheet_frame(bar, page.child(body).into_any_element(), footer)
    }

    /// Root, protection, outcome and failure banners for the Clean workflow.
    fn clean_context_banners(&self, t: &Theme) -> Vec<gpui::AnyElement> {
        let locale = self.session.locale();
        let workbench = self.session.clean_workbench();
        let mut banners = Vec::new();
        if workbench.native_fixture().is_some() {
            banners.push(ui::any(ui::banner(
                t,
                Tone::Warning,
                copy::clean_fixture_notice(locale),
            )));
        }
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
        let project = workbench.selected_scope() != CleanScope::Global;
        let mut configuration = div().flex().flex_col().gap(ui::SPACE_2).child(ui::muted(
            t,
            copy::clean_scope_detail(locale, workbench.selected_scope().id()),
        ));
        if workbench.prompt() == CleanLandingPrompt::ConfirmGlobal {
            return configuration
                .child(self.clean_global_confirm(t, cx))
                .into_any_element();
        }
        if workbench.selected_scope() == CleanScope::All {
            configuration = configuration.child(
                ui::button(
                    t,
                    "clean-include-global",
                    copy::clean_include_global(locale),
                    ButtonKind::Secondary,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.session.clean_request_global_confirmation();
                    cx.notify();
                })),
            );
        }
        if project {
            let roots = self.session.shell().scan_roots();
            configuration = configuration.child(
                div().id("clean-project-roots").flex().flex_col().gap(ui::SPACE_1)
                    .when(roots.is_empty(), |d| d.child(ui::muted(t, copy::pick(locale,
                        "Choose a project folder before scanning. Nothing is scanned automatically.",
                        "先选择项目文件夹，再开始扫描。不会自动扫描。"))))
                    .children(roots.iter().enumerate().map(|(index, root)| {
                        div().id(ElementId::named_usize("clean-saved-root", index))
                            .flex().items_center().gap(ui::SPACE_1).min_w_0()
                            .child(ui::icon("folder", t.accent))
                            .child(div().flex_1().min_w_0().overflow_hidden().text_size(ui::TYPE_BODY)
                                .child(ui::display_path_text(root)))
                    })),
            );
        }
        let has_roots = !self.session.shell().scan_roots().is_empty();
        configuration = configuration.child(
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .items_center()
                .gap(ui::SPACE_2)
                .when(project, |d| {
                    d.child(
                        ui::icon_button(
                            t,
                            "clean-root-choose",
                            "folder",
                            copy::clean_root_choose(locale),
                            if has_roots {
                                ButtonKind::Secondary
                            } else {
                                ButtonKind::Primary
                            },
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.clean_pick_root(cx)))
                        .on_a11y_action(
                            gpui::AccessibleAction::Click,
                            on_view_a11y(cx, |this, cx| this.clean_pick_root(cx)),
                        ),
                    )
                })
                .when(!project || has_roots, |d| {
                    d.child(
                        ui::icon_button(
                            t,
                            "clean-scan-primary",
                            "play",
                            copy::pick(
                                locale,
                                if project {
                                    "Scan projects"
                                } else {
                                    "Scan global caches"
                                },
                                if project {
                                    "扫描项目"
                                } else {
                                    "扫描全局缓存"
                                },
                            ),
                            ButtonKind::Primary,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.clean_start_scope(cx)))
                        .on_a11y_action(
                            gpui::AccessibleAction::Click,
                            on_view_a11y(cx, |this, cx| this.clean_start_scope(cx)),
                        ),
                    )
                }),
        );
        if workbench.prompt() == CleanLandingPrompt::RootRequired {
            configuration = configuration.child(self.clean_root_picker(t, cx));
        }
        configuration.into_any_element()
    }

    fn clean_scope_tabs(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        div()
            .flex()
            .child(
                ui::segmented(t)
                    .id("clean-scopes")
                    .role(gpui::Role::TabList)
                    .aria_label(copy::clean_scope_prompt(locale))
                    .children(
                        [CleanScope::Project, CleanScope::Global, CleanScope::All].map(|scope| {
                            ui::segment(
                                t,
                                ElementId::Name(SharedString::from(format!(
                                    "clean-scope-{}",
                                    scope.id()
                                ))),
                                match scope {
                                    CleanScope::Project => copy::clean_scope_project(locale),
                                    CleanScope::Global => copy::clean_scope_global(locale),
                                    CleanScope::All => copy::clean_scope_all(locale),
                                },
                                self.session.clean_workbench().selected_scope() == scope,
                            )
                            .on_click(
                                cx.listener(move |this, _, _, cx| this.clean_scope(scope, cx)),
                            )
                            .on_a11y_action(
                                gpui::AccessibleAction::Click,
                                on_view_a11y(cx, move |this, cx| this.clean_scope(scope, cx)),
                            )
                        }),
                    ),
            )
            .into_any_element()
    }

    fn clean_root_picker(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let suggestions = self.session.clean_workbench().suggestions().to_vec();
        let rows = suggestions.into_iter().enumerate().map(|(index, root)| {
            let label = root.display().to_string();
            let shown = ui::display_path(&root);
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
            .child(div().flex_1().min_w_0().truncate().child(shown))
            .child(ui::icon("chevron-right", t.text_muted))
        });
        div()
            .flex()
            .flex_col()
            .gap(ui::SPACE_1)
            .child(ui::muted(t, copy::clean_root_suggestions(locale)))
            .children(rows)
            .into_any_element()
    }

    fn clean_global_confirm(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        ui::card(t)
            .w_full()
            .max_w(rems(40.0))
            .gap(ui::SPACE_3)
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
                    .gap(ui::SPACE_1)
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
                        .on_click(cx.listener(|this, _, _, cx| this.clean_confirm_global(cx)))
                        .on_a11y_action(
                            gpui::AccessibleAction::Click,
                            on_view_a11y(cx, |this, cx| this.clean_confirm_global(cx)),
                        ),
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
            .gap(ui::SPACE_3)
            .p(ui::SPACE_5)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(ui::SPACE_3)
                    .child(ui::icon_tile(t, icon, t.accent))
                    .child(
                        div()
                            .flex_1()
                            .text_size(ui::TYPE_TITLE)
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
            .children(workbench.scan_progress().groups.iter().map(
                |(ecosystem, kind, count, capacity)| {
                    ui::muted(
                        t,
                        format!(
                            "{} · {count} · {}",
                            copy::clean_group_title(locale, ecosystem, kind, false),
                            capacity.label(locale)
                        ),
                    )
                },
            ))
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

    fn clean_result_card(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        use devsweep_core::execution::OutcomeStatus;
        let locale = self.session.locale();
        let workbench = self.session.clean_workbench();
        let Some(execution) = workbench.execution() else {
            return ui::any(div());
        };
        let mut card = ui::card(t)
            .id("clean-results")
            .gap(ui::SPACE_3)
            .child(ui::card_header(t, copy::clean_phase_completed(locale)))
            .child(ui::muted(
                t,
                copy::clean_moved_summary(
                    locale,
                    execution.moved_count,
                    &execution.moved.label(locale),
                ),
            ))
            .child(ui::muted(
                t,
                copy::clean_result_status(locale, execution.failed, execution.skipped),
            ))
            .child(ui::banner(
                t,
                if execution.history_incomplete {
                    Tone::Warning
                } else {
                    Tone::Info
                },
                copy::clean_history_evidence(locale, execution.history_incomplete),
            ));
        let visible = self.session.shell().visible_row_ids(Mode::Clean);
        for group in &execution.groups {
            let mut details = div()
                .flex()
                .flex_col()
                .gap(ui::SPACE_1)
                .child(ui::card_header(t, group.group.title(locale)))
                .child(ui::muted(
                    t,
                    copy::clean_moved_summary(
                        locale,
                        group.moved_count,
                        &group.moved.label(locale),
                    ),
                ))
                .child(ui::muted(
                    t,
                    copy::clean_result_status(locale, group.failed, group.skipped),
                ));
            for outcome in visible
                .iter()
                .filter_map(|id| execution.outcome(id))
                .filter(|outcome| {
                    workbench
                        .group_for_row(outcome.target_id.as_str())
                        .map(|entry| entry.id.as_str())
                        .unwrap_or("unmatched")
                        == group.group.id
                })
            {
                let status = match &outcome.status {
                    OutcomeStatus::Succeeded => copy::pick(locale, "Succeeded", "成功").to_string(),
                    OutcomeStatus::Failed { message } => format!(
                        "{}: {}",
                        copy::pick(locale, "Failed", "失败"),
                        crate::clean_workbench::bounded_detail(message)
                    ),
                    OutcomeStatus::Skipped { reason } => format!(
                        "{}: {}",
                        copy::pick(locale, "Skipped", "跳过"),
                        crate::clean_workbench::bounded_detail(reason)
                    ),
                };
                let title = workbench
                    .row(outcome.target_id.as_str())
                    .map(|row| {
                        row.path
                            .as_deref()
                            .map(ui::display_path_text)
                            .unwrap_or_else(|| row.rule_id.clone())
                    })
                    .unwrap_or_else(|| outcome.target_id.as_str().to_string());
                details = details.child(ui::muted(t, format!("{title} · {status}")));
            }
            card = card.child(details);
        }
        let total = self.session.shell().filtered_row_ids(Mode::Clean).len();
        let start = self.session.shell().row_window_start(Mode::Clean);
        card.child(
            div()
                .flex()
                .flex_row()
                .gap(ui::SPACE_2)
                .child(ui::muted(
                    t,
                    format!(
                        "{}–{} / {total}",
                        (start + 1).min(total),
                        (start + visible.len()).min(total)
                    ),
                ))
                .child(
                    ui::button(
                        t,
                        "clean-result-prev",
                        copy::pick(locale, "Previous", "上一页"),
                        ButtonKind::Ghost,
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.session
                            .clean_scroll_rows(-(crate::shell::VISIBLE_ROW_CAPACITY as isize));
                        cx.notify();
                    })),
                )
                .child(
                    ui::button(
                        t,
                        "clean-result-next",
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
        .into_any_element()
    }

    /// Found review puts the targets and staged actions before secondary maps.
    fn clean_review(
        &self,
        t: &Theme,
        row_height: gpui::Pixels,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let locale = self.session.locale();
        let inputs = map_inputs(&self.session);
        let summary = clean_map::summarize(&inputs);
        let stats = div()
            .id("clean-map-stats")
            .flex()
            .flex_row()
            .flex_wrap()
            .gap(ui::SPACE_2)
            .child(ui::stat(
                t,
                copy::clean_map_card_mapped(locale),
                map_capacity(&inputs, None).label(locale),
                copy::clean_map_count(locale, summary.mapped_count),
                t.accent,
            ))
            .child(ui::stat(
                t,
                copy::clean_map_card_recommended(locale),
                map_capacity(&inputs, Some(ReviewBand::Recommended)).label(locale),
                copy::clean_map_count(locale, summary.recommended_count),
                t.success,
            ))
            .child(ui::stat(
                t,
                copy::clean_map_card_review(locale),
                map_capacity(&inputs, Some(ReviewBand::NeedsReview)).label(locale),
                copy::clean_map_count(locale, summary.review_count),
                t.warning,
            ))
            .child(ui::stat(
                t,
                copy::clean_map_card_blocked(locale),
                map_capacity(&inputs, Some(ReviewBand::NotSelectable)).label(locale),
                copy::clean_map_count(locale, summary.blocked_count),
                t.text_muted,
            ));
        let workbench = self.session.clean_workbench();
        let overview = ui::card(t)
            .gap(ui::SPACE_2)
            .children(workbench.groups().iter().map(|group| {
                ui::any(
                    div()
                        .flex()
                        .flex_row()
                        .justify_between()
                        .gap(ui::SPACE_2)
                        .child(ui::muted(t, group.title(locale)))
                        .child(ui::muted(t, group.capacity.label(locale))),
                )
            }))
            .child(ui::muted(t, copy::clean_review_only_detail(locale)));
        div()
            .flex()
            .flex_col()
            .gap(ui::SPACE_4)
            .child(if workbench.reviewing() {
                self.clean_rows_card(t, row_height, cx)
            } else {
                overview.into_any_element()
            })
            .child(stats)
            .when(
                !self.session.shell().row_query(Mode::Clean).is_empty(),
                |d| d.child(ui::muted(t, copy::clean_map_filtered_note(locale))),
            )
            .when(disk_map_active(&self.session), |d| {
                d.child(self.clean_disk_map(t, cx))
            })
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
            .w(px(MAP_WIDTH))
            .max_w_full()
            .h(px(MAP_HEIGHT))
            .overflow_hidden()
            .rounded(ui::RADIUS_ITEM)
            .bg(rgb(t.surface_alt));
        if map.groups.is_empty() {
            canvas = canvas.child(
                div()
                    .p(ui::SPACE_3)
                    .child(ui::muted(t, copy::clean_map_empty(locale))),
            );
        }
        // Tile labels are rem sized, so their fit thresholds follow the text
        // scale while the map geometry stays in pixels.
        let text_scale = f32::from(self.session.text_scale()) / 100.0;
        for (group_index, group) in map.groups.iter().enumerate() {
            let title = if group.other {
                copy::clean_map_other(locale, group.represented)
            } else {
                workbench
                    .groups()
                    .iter()
                    .find(|review| review.id == group.title)
                    .map(|review| review.title(locale))
                    .unwrap_or_else(|| group.title.clone())
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
                // The group box is the whole group rectangle. Each tile
                // insets itself by one pixel, so a box inset as well would
                // clip the right and bottom edge of its last tiles.
                .absolute()
                .left(gpui::relative(group.x / MAP_WIDTH))
                .top(px(group.y))
                .w(gpui::relative(group.width.max(1.0) / MAP_WIDTH))
                .h(px(group.height.max(1.0)))
                .rounded(ui::RADIUS_CHECK)
                .overflow_hidden()
                .aria_label(SharedString::from(group_label));
            if group.height >= 30.0 && group.width >= 60.0 {
                node = node.child(
                    // The band has a fixed pixel height while the label is
                    // rem sized: one line of exactly the band height keeps
                    // the label inside it at every text scale.
                    div()
                        .absolute()
                        .left(px(6.0))
                        .top(px(0.0))
                        .right(px(4.0))
                        .h(px(clean_map::GROUP_HEADER as f32))
                        .line_height(px(clean_map::GROUP_HEADER as f32))
                        .truncate()
                        .text_size(ui::TYPE_META)
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
                    .left(gpui::relative(x / group.width.max(1.0)))
                    .top(px(y))
                    .w(gpui::relative(w / group.width.max(1.0)))
                    .h(px(h))
                    .overflow_hidden()
                    .rounded(ui::RADIUS_CHECK)
                    // Horizontal padding only: the smallest spacing step on
                    // all four sides would leave a short tile no line height.
                    .px(ui::SPACE_1)
                    .bg(t.tint(hue, if selected { 0x30 } else { 0x18 }))
                    .border_1()
                    .border_color(if selected { rgb(t.text) } else { rgb(hue) })
                    .aria_label(SharedString::from(label));
                if let Some(id) = leaf.id.clone() {
                    tile = tile
                        .role(gpui::Role::Button)
                        .cursor_pointer()
                        .hover(|s| s.border_color(rgb(t.text)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.session.clean_click_row(&id);
                            cx.notify();
                        }));
                }
                if w >= 56.0 * text_scale && h >= 24.0 * text_scale {
                    tile = tile.child(
                        div()
                            .truncate()
                            .text_size(ui::TYPE_META)
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(t.text))
                            .child(SharedString::from(leaf_title)),
                    );
                    if h >= 40.0 * text_scale {
                        tile = tile.child(
                            div()
                                .text_size(ui::TYPE_META)
                                .text_color(rgb(t.text))
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
            .gap(ui::SPACE_1)
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
                    .gap(ui::SPACE_1)
                    .child(
                        div()
                            .size(rems(0.7))
                            .rounded(ui::RADIUS_CHECK)
                            .bg(rgb(t.category(category_index(*category)))),
                    )
                    .child(
                        div()
                            .text_size(ui::TYPE_META)
                            .text_color(rgb(t.text_muted))
                            .child(copy::clean_map_category(locale, category.as_str())),
                    )
            }));
        ui::card(t)
            .id("clean-disk-map")
            .role(gpui::Role::Group)
            .aria_label(copy::clean_map_title(locale))
            .gap(ui::SPACE_3)
            .child(
                div()
                    .flex()
                    .flex_col()
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
                    .gap(ui::SPACE_4)
                    .child(canvas)
                    .child(legend),
            )
            .into_any_element()
    }

    fn clean_rows_card(
        &self,
        t: &Theme,
        row_height: gpui::Pixels,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
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
            .gap(ui::SPACE_1)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(ui::SPACE_1)
                    .w(rems(20.0))
                    .max_w_full()
                    .child(ui::icon("search", t.text_muted))
                    .child(div().flex_1().min_w_0().child(self.search.clone())),
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
            .max_h(row_height)
            .overflow_y_scroll()
            .flex()
            .flex_col();
        let mut last_group: Option<&str> = None;
        for id in shell.visible_row_ids(Mode::Clean) {
            let Some(row) = workbench.row(id) else {
                continue;
            };
            if let Some(group) = workbench.group_for_row(id)
                && last_group != Some(group.id.as_str())
            {
                rows = rows.child(self.clean_group_header(t, group, cx));
                last_group = Some(group.id.as_str());
            }
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
                .pt(ui::SPACE_1)
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
                        .gap(ui::SPACE_1)
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
            .gap(ui::SPACE_3)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(ui::card_header(t, copy::clean_map_recommendations(locale)))
                    .child(ui::muted(t, copy::clean_map_recommendations_hint(locale))),
            )
            .child(toolbar)
            .child(ui::divider(t))
            .child(rows)
            .children(pager)
            .into_any_element()
    }

    fn clean_group_header(
        &self,
        t: &Theme,
        group: &crate::clean_workbench::CleanGroup,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        use crate::clean_workbench::GroupSelection;
        let locale = self.session.locale();
        let toggled = match group.selection {
            GroupSelection::None => gpui::Toggled::False,
            GroupSelection::Mixed => gpui::Toggled::Mixed,
            GroupSelection::All => gpui::Toggled::True,
        };
        let enabled = !group.selectable_ids.is_empty()
            && self.session.clean_workbench().phase() == CleanPhase::Found;
        let id = group.id.clone();
        let accessible_id = id.clone();
        let label = format!(
            "{} · {} · {} {}",
            group.title(locale),
            group.capacity.label(locale),
            group.ids.len(),
            copy::pick(locale, "targets in full scope", "个目标（完整范围）")
        );
        let selection_label = match group.selection {
            GroupSelection::None => copy::pick(locale, "None selected", "未选"),
            GroupSelection::Mixed => copy::pick(locale, "Partially selected", "部分选中"),
            GroupSelection::All => copy::pick(locale, "All selected", "全选"),
        };
        let label = format!(
            "{label} · {selection_label} · {}/{}",
            group.selected_count,
            group.selectable_ids.len()
        );
        ui::button(
            t,
            ElementId::Name(format!("clean-group-{}", group.id).into()),
            label,
            if enabled {
                ButtonKind::Secondary
            } else {
                ButtonKind::Disabled
            },
        )
        .accessibility_id(SharedString::from(format!("clean-group-{}", group.id)))
        .role(gpui::Role::CheckBox)
        .aria_toggled(toggled)
        .child(ui::checkbox_state(t, toggled, enabled))
        .when(enabled, |button| {
            button
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.session.clean_toggle_group(&id);
                    cx.notify();
                }))
                .on_a11y_action(
                    gpui::AccessibleAction::Click,
                    on_view_a11y(cx, move |this, cx| {
                        this.session.clean_toggle_group(&accessible_id);
                        cx.notify();
                    }),
                )
        })
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
            Some(path) => ui::display_path_text(path),
            None => copy::clean_row_global(locale).to_string(),
        };
        let element_id = SharedString::from(format!("clean-row-{}", row.id));
        let clicked = row.id.clone();
        let mut badges = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap(ui::SPACE_1)
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
                    .child(ui::two_line(t, title, subtitle))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .flex_wrap()
                            .items_center()
                            .gap(ui::SPACE_1)
                            .child(badges)
                            .child(
                                div()
                                    .text_size(ui::TYPE_META)
                                    .text_color(rgb(t.text_muted))
                                    .child(copy::clean_row_rebuild_cost(locale, &row.rule_id)),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(ui::TYPE_TITLE)
                    .font_features(ui::tabular_numerals())
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(copy::clean_bytes_short(row.bytes)),
            )
            .into_any_element()
    }

    /// Selection summary and the staged preview/confirm actions.
    fn clean_action_bar(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let workbench = self.session.clean_workbench();
        let selected_capacity = workbench.selected_capacity();
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
            .gap(ui::SPACE_1)
            .p(ui::SPACE_3)
            .rounded(ui::RADIUS_ITEM)
            .bg(rgb(t.surface_alt))
            .when(t.high_contrast, |d| {
                d.border_1().border_color(rgb(t.border))
            })
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(ui::SPACE_2)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w(rems(12.0))
                            .child(
                                div()
                                    .text_size(ui::TYPE_TITLE)
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(selected_capacity.label(locale)),
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
                                ButtonKind::Danger
                            } else {
                                ButtonKind::Disabled
                            },
                        )
                        .when(ready, |b| {
                            b.on_click(cx.listener(|this, _, _, cx| this.clean_confirm_execute(cx)))
                        }),
                    ),
            )
            .when_some(workbench.preview(), |bar, preview| {
                bar.child(preview_status(
                    t,
                    locale,
                    "clean-preview-status",
                    ready,
                    copy::clean_preview_digest_line(locale, &preview.digest, preview.count),
                ))
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
        let bar = self.page_title(t, copy::clean_ai_title(locale), actions);
        let mut page = div()
            .id("clean-ai-controls")
            .role(gpui::Role::Group)
            .aria_label(copy::clean_ai_title(locale))
            .flex()
            .flex_col()
            .gap(ui::SPACE_4)
            .child(ui::muted(t, copy::clean_ai_subtitle(locale)))
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
                copy::clean_ai_home_line(locale, &ui::display_path(home)),
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
                        .gap(ui::SPACE_3)
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(ui::SPACE_3)
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
                        .gap(ui::SPACE_3)
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
        sheet_frame(bar, page.into_any_element(), None)
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
            .flex_col();
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
            .gap(ui::SPACE_3)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(ui::SPACE_2)
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
                            .gap(ui::SPACE_1)
                            .w(rems(16.0))
                            .max_w_full()
                            .child(ui::icon("search", t.text_muted))
                            .child(div().flex_1().min_w_0().child(self.search.clone())),
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
                    .gap(ui::SPACE_1)
                    .p(ui::SPACE_3)
                    .rounded(ui::RADIUS_ITEM)
                    .bg(rgb(t.surface_alt))
                    .when(t.high_contrast, |d| {
                        d.border_1().border_color(rgb(t.border))
                    })
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
                                ButtonKind::Danger
                            } else {
                                ButtonKind::Disabled
                            },
                        )
                        .when(ready, |b| {
                            b.on_click(
                                cx.listener(|this, _, _, cx| this.clean_ai_confirm_execute(cx)),
                            )
                        }),
                    ),
            )
            .when_some(view.preview(), |c, preview| {
                c.child(preview_status(
                    t,
                    locale,
                    "clean-ai-preview-status",
                    ready,
                    copy::clean_ai_digest_line(locale, &preview.digest, preview.count),
                ))
            })
            .into_any_element()
    }

    fn clean_ai_recovery(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let view = self.session.clean_ai();
        let mut list = div().flex().flex_col().gap(ui::SPACE_1);
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
                    .gap(ui::SPACE_1)
                    .px(ui::SPACE_2)
                    .py(ui::SPACE_1)
                    .rounded(ui::RADIUS_ITEM)
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
            .gap(ui::SPACE_2)
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
