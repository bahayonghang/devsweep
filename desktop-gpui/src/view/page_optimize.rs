use super::*;
use devsweep_core::optimize::{
    BatchOutcome, BatchReadiness, MaintenanceActionClass, published_entries,
};

impl ShellView {
    pub(super) fn optimize_page(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let zh = locale == copy::Locale::ZhCn;
        let pick = |en, zh_text| copy::pick(locale, en, zh_text);
        let batch = self.session.batch();
        let generation = self.session.batch_generation();
        let count = batch.selected.len();
        let mut actions = Vec::new();
        let current = batch.current();
        let status = if let Some(index) = current {
            let title = batch
                .selected
                .get(index)
                .map(|id| copy::optimize_task_title(locale, id))
                .unwrap_or_default();
            if zh {
                format!("正在运行 {}/{} · {title}", index + 1, count)
            } else {
                format!("Running {} of {} · {title}", index + 1, count)
            }
        } else if batch.previewing() {
            pick("Checking prerequisites…", "正在核对前置条件…").to_string()
        } else if batch.review.is_some() {
            pick("Review maintenance plan", "审阅维护计划").to_string()
        } else if batch.report.is_some() {
            pick("Maintenance results", "维护结果").to_string()
        } else {
            let catalogue = copy::optimize_catalogue_status(
                locale,
                published_entries().len(),
                published_entries()
                    .iter()
                    .filter(|e| e.action_class == MaintenanceActionClass::Execute)
                    .count(),
            );
            format!(
                "{catalogue} · {}",
                pick("Disk health is read-only", "磁盘健康为只读检查")
            )
        };
        let button = if current.is_some() {
            ui::button(
                t,
                "optimize-batch-cancel",
                pick("Cancel remaining", "取消余下项目"),
                ButtonKind::Secondary,
            )
            .accessibility_id("optimize-batch-cancel")
            .on_click(cx.listener(move |this, _, _, cx| {
                this.session.cancel_batch(generation);
                cx.notify();
            }))
            .on_a11y_action(
                gpui::AccessibleAction::Click,
                on_view_a11y(cx, move |this, cx| {
                    this.session.cancel_batch(generation);
                    cx.notify();
                }),
            )
        } else if batch.review.is_some() {
            ui::button(
                t,
                "optimize-batch-confirm",
                pick("Confirm maintenance", "确认维护"),
                ButtonKind::Primary,
            )
            .accessibility_id("optimize-batch-confirm")
            .on_click(cx.listener(move |this, _, _, cx| {
                this.session.confirm_batch(generation);
                cx.notify();
            }))
            .on_a11y_action(
                gpui::AccessibleAction::Click,
                on_view_a11y(cx, move |this, cx| {
                    this.session.confirm_batch(generation);
                    cx.notify();
                }),
            )
        } else {
            ui::button(
                t,
                "optimize-batch-review",
                pick("Review plan", "审阅计划"),
                if count == 0 || batch.previewing() {
                    ButtonKind::Disabled
                } else {
                    ButtonKind::Primary
                },
            )
            .accessibility_id("optimize-batch-review")
            .on_click(cx.listener(|this, _, _, cx| {
                this.session.review_batch();
                cx.notify();
            }))
            .on_a11y_action(
                gpui::AccessibleAction::Click,
                on_view_a11y(cx, |this, cx| {
                    this.session.review_batch();
                    cx.notify();
                }),
            )
        };
        actions.push(button.into_any_element());
        let total = if zh {
            format!("已选 {count} 项")
        } else {
            format!("{count} selected")
        };
        let bar = page_bar(t, Some(status), Some(total), actions);
        let mut body = div().flex().flex_col().gap(ui::SPACE_2);
        if batch.fixture {
            body = body.child(
                ui::muted(t, pick("Isolated maintenance fixture", "隔离维护夹具"))
                    .id("optimize-isolated-fixture")
                    .role(gpui::Role::Status)
                    .accessibility_id("optimize-isolated-fixture"),
            );
        }
        if let Some(error) = batch.error {
            body = body.child(ui::banner(t, Tone::Warning, error.label(locale)));
        }
        if let Some((_, review)) = &batch.review {
            let mut area = ui::card(t)
                .id("optimize-batch-preview")
                .role(gpui::Role::Group)
                .accessibility_id("optimize-batch-preview")
                .aria_label(pick("Review maintenance plan", "审阅维护计划"))
                .gap(ui::SPACE_1);
            for item in &review.operations {
                let status = match item.state {
                    BatchReadiness::Ready { .. } => pick("Ready", "就绪").to_string(),
                    BatchReadiness::Skipped { reason } => {
                        format!("{} · {}", pick("Skipped", "已跳过"), reason.label(zh))
                    }
                };
                let key = SharedString::from(format!("optimize-preview-{}", item.operation_id));
                let title = copy::optimize_task_title(locale, &item.operation_id);
                let label = SharedString::from(format!("{title} · {status}"));
                area = area.child(
                    ui::two_line(t, title, status)
                        .id(ElementId::Name(key.clone()))
                        .role(gpui::Role::ListItem)
                        .accessibility_id(key)
                        .aria_label(label),
                );
            }
            body = body.child(
                area.child(
                    ui::muted(t, review.digest.clone())
                        .id("optimize-preview-digest")
                        .role(gpui::Role::Label)
                        .aria_label(SharedString::from(review.digest.clone())),
                ),
            );
        }
        if let Some(report) = &batch.report {
            let mut area = ui::card(t)
                .id("optimize-batch-results")
                .role(gpui::Role::Group)
                .accessibility_id("optimize-batch-results")
                .aria_label(pick("Maintenance results", "维护结果"))
                .gap(ui::SPACE_1);
            for result in &report.outcomes {
                let (icon, tone) = match result.outcome {
                    BatchOutcome::Succeeded => ("check-circle", Tone::Accent),
                    BatchOutcome::Skipped { .. } => ("info", Tone::Neutral),
                    BatchOutcome::Failed { .. } | BatchOutcome::UnknownAfterDispatch { .. } => {
                        ("alert", Tone::Warning)
                    }
                };
                let key = SharedString::from(format!("optimize-result-{}", result.catalogue_id));
                let title = copy::optimize_task_title(locale, &result.catalogue_id);
                let status = result.outcome.label(zh);
                let label = SharedString::from(format!("{title} · {status}"));
                area = area.child(
                    div()
                        .id(ElementId::Name(key.clone()))
                        .role(gpui::Role::ListItem)
                        .accessibility_id(key)
                        .aria_label(label)
                        .flex()
                        .items_center()
                        .gap(ui::SPACE_2)
                        .child(ui::icon(icon, t.tone(tone)))
                        .child(ui::two_line(t, title, status)),
                );
            }
            body = body.child(
                area.child(
                    ui::muted(t, report.batch_id.clone())
                        .id("optimize-result-history-id")
                        .role(gpui::Role::Label)
                        .aria_label(SharedString::from(report.batch_id.clone())),
                ),
            );
        }
        let mut rows = ui::card(t).gap(ui::SPACE_1).child(ui::card_header(
            t,
            pick("Maintenance operations", "维护项目"),
        ));
        for entry in published_entries()
            .iter()
            .filter(|e| e.action_class == MaintenanceActionClass::Execute)
        {
            let id = entry.id;
            let checked = batch.selected.iter().any(|s| s == id);
            let key = SharedString::from(format!("optimize-check-{id}"));
            let title = copy::optimize_task_title(locale, id);
            let detail = match id {
                "dns.flush" => pick("Refresh the DNS resolver cache", "刷新 DNS 解析缓存"),
                "maintenance.clock_probe" => pick(
                    "Request three samples from time.windows.com",
                    "向 time.windows.com 请求三次采样",
                ),
                "maintenance.clock_sync" => pick(
                    "Resynchronize the system clock · Administrator",
                    "重新同步系统时间 · 管理员权限",
                ),
                "maintenance.retrim" => pick(
                    "Retrim an eligible SSD · Administrator",
                    "对符合条件的固态硬盘执行重新优化 · 管理员权限",
                ),
                "maintenance.image_scan" => pick(
                    "Scan the Windows image · Administrator",
                    "扫描 Windows 映像 · 管理员权限",
                ),
                "maintenance.image_repair" => pick(
                    "Repair a verified repairable image · Administrator",
                    "修复已证实可修复的映像 · 管理员权限",
                ),
                _ => pick(
                    "Back up and remove missing-target startup entries · Administrator",
                    "备份并移除目标已丢失的启动项 · 管理员权限",
                ),
            };
            let enabled = !batch.running();
            rows = rows.child(
                ui::list_row(t, ElementId::Name(key.clone()), checked, false)
                    .accessibility_id(key)
                    .role(gpui::Role::CheckBox)
                    .aria_label(SharedString::from(format!("{title} · {detail}")))
                    .aria_toggled(if checked {
                        gpui::Toggled::True
                    } else {
                        gpui::Toggled::False
                    })
                    .when(!enabled, |row| {
                        row.a11y_synthetic_children(|tree| tree.parent_node().set_disabled())
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.session.toggle_batch_item(id);
                        cx.notify();
                    }))
                    .on_a11y_action(
                        gpui::AccessibleAction::Click,
                        on_view_a11y(cx, move |this, cx| {
                            this.session.toggle_batch_item(id);
                            cx.notify();
                        }),
                    )
                    .child(ui::checkbox(t, checked, enabled))
                    .child(ui::two_line(t, title, detail)),
            );
        }
        body = body.child(rows);
        let mut guides = ui::card(t).gap(ui::SPACE_1).child(
            ui::button(
                t,
                "optimize-guidance",
                if batch.guidance_open {
                    pick("Hide guidance and settings", "收起指引与系统设置")
                } else {
                    pick("Guidance and system settings", "指引与系统设置")
                },
                ButtonKind::Ghost,
            )
            .accessibility_id("optimize-guidance")
            .on_click(cx.listener(|this, _, _, cx| {
                this.session.toggle_batch_guidance();
                cx.notify();
            }))
            .on_a11y_action(
                gpui::AccessibleAction::Click,
                on_view_a11y(cx, |this, cx| {
                    this.session.toggle_batch_guidance();
                    cx.notify();
                }),
            ),
        );
        if batch.guidance_open {
            for entry in published_entries().iter().filter(|e| {
                matches!(
                    e.action_class,
                    MaintenanceActionClass::Guidance | MaintenanceActionClass::SettingsHandoff
                )
            }) {
                let id = entry.id;
                let key = SharedString::from(format!("optimize-guide-{id}"));
                let detail = if entry.action_class == MaintenanceActionClass::SettingsHandoff {
                    copy::optimize_class_label(locale, "settings_handoff")
                } else {
                    copy::optimize_class_label(locale, "guidance")
                };
                guides = guides.child(
                    ui::button(
                        t,
                        ElementId::Name(key.clone()),
                        format!("{} · {detail}", copy::optimize_task_title(locale, id)),
                        ButtonKind::Ghost,
                    )
                    .accessibility_id(key)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.session
                            .activate_mode_row(Mode::Optimize, id.to_string());
                        cx.notify();
                    }))
                    .on_a11y_action(
                        gpui::AccessibleAction::Click,
                        on_view_a11y(cx, move |this, cx| {
                            this.session
                                .activate_mode_row(Mode::Optimize, id.to_string());
                            cx.notify();
                        }),
                    ),
                );
            }
            guides = guides.children(self.workbench_review_card(t));
        }
        body = body.child(guides);
        sheet_frame(bar, body.into_any_element(), None)
    }
}
