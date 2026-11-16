//! Fixed, keyboard reachable confirmation shared by the authorized host actions.
use super::*;
use crate::host_review::Phase;

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
        let ready = review.phase == Phase::Ready && !self.session.host_action_busy();
        let restore = review.results.iter().any(|result| result.reverse.is_some());
        let process = review
            .intents
            .iter()
            .any(|intent| matches!(intent, crate::host_review::Intent::Process { .. }));
        Some(
            div()
                .id("host-action-review")
                .role(gpui::Role::Group)
                .aria_label(copy::pick(locale, "Review host changes", "审阅主机更改"))
                .flex()
                .flex_col()
                .gap(ui::SPACE_2)
                .p(ui::SPACE_3)
                .rounded(ui::RADIUS_WELL)
                .bg(rgb(t.surface_alt))
                .children(
                    self.session
                        .host_review_lines()
                        .into_iter()
                        .map(|line| ui::muted(t, line)),
                )
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
                                    if process {
                                        copy::pick(locale, "Confirm ending process", "确认结束进程")
                                    } else {
                                        copy::pick(locale, "Confirm these changes", "确认这些更改")
                                    },
                                    if process {
                                        ButtonKind::Danger
                                    } else {
                                        ButtonKind::Primary
                                    },
                                )
                                .accessibility_id("host-action-confirm")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.session.host_confirm(true);
                                    cx.notify();
                                }))
                                .on_a11y_action(
                                    gpui::AccessibleAction::Click,
                                    on_view_a11y(cx, |this, cx| {
                                        this.session.host_confirm(true);
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
