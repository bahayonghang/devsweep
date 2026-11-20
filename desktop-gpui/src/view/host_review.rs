//! Fixed, keyboard reachable confirmation shared by the authorized host actions.
use super::*;
use crate::host_review::Phase;

#[cfg(all(windows, test))]
#[path = "host_receipt_window.rs"]
pub(crate) mod receipt;

impl ShellView {
    pub(super) fn host_review_bar(
        &self,
        t: &Theme,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let review = self.session.host_review();
        if review.phase == Phase::Idle && !self.session.host_action_busy() {
            return None;
        }
        let locale = self.session.locale();
        let generation = review.generation;
        let ready = review.phase == Phase::Ready && !self.session.host_action_busy();
        let leftovers = self.session.can_review_leftovers();
        let moving = review.intents.iter().any(|intent| matches!(intent, crate::host_review::Intent::Leftovers(row) if row.candidate.is_some()));
        let restore = review.results.iter().any(|result| result.reverse.is_some());
        let uninstall = review
            .intents
            .iter()
            .any(|intent| matches!(intent, crate::host_review::Intent::Uninstall(_)));
        let process = review
            .intents
            .iter()
            .any(|intent| matches!(intent, crate::host_review::Intent::Process { .. }));
        Some(
            ui::overlay(t)
                .id("host-action-review")
                .role(gpui::Role::Group)
                .aria_label(copy::pick(locale, "Review host changes", "审阅主机更改"))
                .gap(ui::SPACE_2)
                .p(ui::SPACE_3)
                .children(
                    self.session
                        .host_review_lines()
                        .into_iter()
                        .map(|line| ui::muted(t, line)),
                )
                .when(review.phase == Phase::Choosing, |bar| {
                    let mut choices = div()
                        .id("leftover-choices")
                        .max_h(px(132.))
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .gap(ui::SPACE_1);
                    if let [
                        crate::host_review::Prepared {
                            plan: crate::host_review::Plan::LeftoverChoices(items),
                            ..
                        },
                    ] = review.prepared.as_slice()
                    {
                        for (index, item) in items.iter().enumerate() {
                            let label = format!(
                                "{} · {}",
                                copy::pick(locale, "Review directory", "审阅目录"),
                                item.path.display()
                            );
                            choices = choices.child(
                                ui::button(
                                    t,
                                    ("host-leftover-choice", index),
                                    label,
                                    ButtonKind::Secondary,
                                )
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.session.choose_leftover(generation, index);
                                    cx.notify();
                                }))
                                .on_a11y_action(
                                    gpui::AccessibleAction::Click,
                                    on_view_a11y(cx, move |this, cx| {
                                        this.session.choose_leftover(generation, index);
                                        cx.notify();
                                    }),
                                ),
                            );
                        }
                    }
                    bar.child(choices)
                })
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .justify_end()
                        .gap(ui::SPACE_1)
                        .child(
                            ui::button(
                                t,
                                "host-action-cancel",
                                copy::pick(
                                    locale,
                                    "Dismiss / cancel remaining",
                                    "关闭/取消剩余项目",
                                ),
                                ButtonKind::Secondary,
                            )
                            .accessibility_id("host-action-cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.session.host_cancel();
                                cx.notify();
                            }))
                            .on_a11y_action(
                                gpui::AccessibleAction::Click,
                                on_view_a11y(cx, |this, cx| {
                                    this.session.host_cancel();
                                    cx.notify();
                                }),
                            ),
                        )
                        .when(leftovers, |row| {
                            row.child(
                                ui::button(
                                    t,
                                    "host-leftovers",
                                    copy::pick(locale, "Review leftovers", "审阅卸载残留"),
                                    ButtonKind::Secondary,
                                )
                                .accessibility_id("host-leftovers")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.session.review_leftovers(generation);
                                    cx.notify();
                                }))
                                .on_a11y_action(
                                    gpui::AccessibleAction::Click,
                                    on_view_a11y(cx, move |this, cx| {
                                        this.session.review_leftovers(generation);
                                        cx.notify();
                                    }),
                                ),
                            )
                        })
                        .when(restore, |row| {
                            row.child(
                                ui::button(
                                    t,
                                    "host-action-restore",
                                    copy::pick(locale, "Review reverse change", "审阅恢复更改"),
                                    ButtonKind::Secondary,
                                )
                                .accessibility_id("host-action-restore")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.session.host_preview_restore();
                                    cx.notify();
                                }))
                                .on_a11y_action(
                                    gpui::AccessibleAction::Click,
                                    on_view_a11y(cx, |this, cx| {
                                        this.session.host_preview_restore();
                                        cx.notify();
                                    }),
                                ),
                            )
                        })
                        .when(ready, |row| {
                            row.child(
                                ui::button(
                                    t,
                                    "host-action-confirm",
                                    if moving {
                                        copy::pick(
                                            locale,
                                            "Confirm move to Recycle Bin",
                                            "确认移入回收站",
                                        )
                                    } else if uninstall {
                                        copy::pick(locale, "Confirm uninstall", "确认卸载")
                                    } else if process {
                                        copy::pick(locale, "Confirm ending process", "确认结束进程")
                                    } else {
                                        copy::pick(locale, "Confirm these changes", "确认这些更改")
                                    },
                                    if process || uninstall || moving {
                                        ButtonKind::Danger
                                    } else {
                                        ButtonKind::Primary
                                    },
                                )
                                .accessibility_id("host-action-confirm")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if this.session.host_review().generation == generation {
                                        this.session.host_confirm(true);
                                    }
                                    cx.notify();
                                }))
                                .on_a11y_action(
                                    gpui::AccessibleAction::Click,
                                    on_view_a11y(cx, move |this, cx| {
                                        if this.session.host_review().generation == generation {
                                            this.session.host_confirm(true);
                                        }
                                        cx.notify();
                                    }),
                                ),
                            )
                        }),
                )
                .into_any_element(),
        )
    }
}
