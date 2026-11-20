//! The notification panel observes the main shell's published Status snapshot.
use super::*;
use crate::tray::{self, Event};
use devsweep_core::history::HistoryTotalsV1;
use gpui::{Global, WindowHandle, WindowKind};
use std::rc::Rc;
use windows::Win32::Foundation::POINT;

pub(super) struct Runtime {
    pub(super) tray: Rc<tray::Tray>,
    main: WindowHandle<ShellView>,
    panel: Option<WindowHandle<Panel>>,
}
impl Global for Runtime {}

pub(super) fn fixture_enabled() -> bool {
    #[cfg(test)]
    {
        std::env::var_os("DEVSWEEP_NATIVE_TRAY_RECEIPT").is_some()
    }
    #[cfg(not(test))]
    {
        false
    }
}

pub(super) fn install(main: WindowHandle<ShellView>, cx: &mut App) {
    let (sender, receiver) = async_channel::unbounded();
    let locale = main.read(cx).expect("main shell").session.locale();
    #[cfg(test)]
    let notify = if fixture_enabled() {
        receipt::fixture_notification
    } else {
        tray::notify_shell
    };
    #[cfg(not(test))]
    let notify = tray::notify_shell;
    let tray = Rc::new(tray::Tray::new(sender, locale, notify));
    main.update(cx, |_, window, _| tray.watch_main(window))
        .expect("watch main window");
    cx.set_global(Runtime {
        tray,
        main,
        panel: None,
    });
    cx.on_window_closed(move |cx, id| {
        if main.window_id() == id {
            cx.quit();
        } else if cx
            .global::<Runtime>()
            .panel
            .is_some_and(|p| p.window_id() == id)
        {
            cx.global_mut::<Runtime>().panel = None;
            let _ = main.update(cx, |shell, _, cx| {
                shell.session.status_set_tray_visible(false);
                cx.notify();
            });
        }
    })
    .detach();
    cx.spawn(async move |cx| {
        while let Ok(event) = receiver.recv().await {
            cx.update(|cx| event_received(event, cx));
        }
    })
    .detach();
}

fn event_received(event: Event, cx: &mut App) {
    let runtime = cx.global::<Runtime>();
    let main = runtime.main;
    let panel = runtime.panel;
    match event {
        Event::Toggle(anchor) => {
            if let Some(panel) = panel {
                let _ = panel.update(cx, |_, window, _| window.remove_window());
            } else {
                open_panel(main, anchor, cx);
            }
        }
        Event::Open | Event::Awake => {
            runtime.tray.set_tucked(false);
            let _ = main.update(cx, |shell, window, cx| {
                tray::restore(window);
                window.activate_window();
                if matches!(event, Event::Awake) {
                    shell.session.show_mode(Mode::Status);
                    shell.session.keep_awake_preview_toggle();
                }
                cx.notify();
            });
        }
        Event::Exit => {
            if main
                .read(cx)
                .is_ok_and(|shell| shell.session.host_action_busy())
            {
                event_received(Event::Open, cx);
            } else {
                // Destroy HWNDs while GPUI and its accessibility provider are
                // still alive. The normal main-window close observer quits.
                if let Some(panel) = panel {
                    let _ = panel.update(cx, |_, window, _| window.remove_window());
                }
                let _ = main.update(cx, |_, window, _| window.remove_window());
            }
        }
        Event::Visibility => {
            let _ = main.update(cx, |shell, window, cx| {
                let selected = shell.session.shell().mode() == Mode::Status
                    && shell.session.shell().surface() == Surface::Mode;
                shell
                    .session
                    .status_set_main_visible(selected && tray::main_visible(window));
                if let Some(gen_id) = shell.session.take_status_poll_request() {
                    shell.start_status_poll(gen_id, cx);
                }
                cx.notify();
            });
        }
    }
}

fn open_panel(main: WindowHandle<ShellView>, anchor: POINT, cx: &mut App) {
    let Ok(shell) = main.update(cx, |_, _, cx| cx.entity()) else {
        return;
    };
    let bounds = Bounds::centered(None, size(px(408.), px(556.)), cx);
    let panel = cx.open_window(
        WindowOptions {
            kind: WindowKind::PopUp,
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: None,
            is_movable: false,
            is_resizable: false,
            is_minimizable: false,
            focus: true,
            show: true,
            ..Default::default()
        },
        move |window, cx| {
            window.set_window_title("DevSweep Status");
            cx.new(|cx| {
                let focus = cx.focus_handle();
                window.focus(&focus, cx);
                cx.observe(&shell, |_, _, cx| cx.notify()).detach();
                cx.observe_window_activation(window, |_, window, _| {
                    if !window.is_window_active() {
                        window.remove_window();
                    }
                })
                .detach();
                Panel {
                    shell,
                    focus,
                    totals: None,
                }
            })
        },
    );
    let Ok(panel) = panel else {
        event_received(Event::Open, cx);
        return;
    };
    cx.global_mut::<Runtime>().panel = Some(panel);
    let _ = main.update(cx, |shell, _, cx| {
        shell.session.status_set_tray_visible(true);
        if let Some(gen_id) = shell.session.take_status_poll_request() {
            shell.start_status_poll(gen_id, cx);
        }
        cx.notify();
    });
    let _ = panel.update(cx, |_, window, cx| {
        tray::place_panel(window, anchor);
        window.activate_window();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    #[cfg(test)]
                    if fixture_enabled() {
                        return Some(receipt::totals());
                    }
                    devsweep_core::history::history_totals().ok()
                })
                .await;
            let _ = this.update(cx, |panel, cx| {
                panel.totals = Some(result);
                cx.notify();
            });
        })
        .detach();
    });
}

struct Panel {
    shell: Entity<ShellView>,
    focus: FocusHandle,
    totals: Option<Option<HistoryTotalsV1>>,
}

impl Render for Panel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let shell = self.shell.read(cx);
        let locale = shell.session.locale();
        let t = shell.material.theme(shell.session.theme());
        window.set_rem_size(px(16. * f32::from(shell.session.text_scale()) / 100.));
        let mut body = div()
            .id("tray-panel-body")
            .overflow_y_scroll()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap(ui::SPACE_2)
            .p(ui::SPACE_3)
            .line_height(rems(1.25));
        let caption = |label: &'static str| {
            div()
                .text_size(ui::TYPE_META)
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(t.muted))
                .child(label)
        };
        let row = |label: String, value: String| {
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap(ui::SPACE_1)
                .child(div().flex_1().min_w_0().truncate().child(label))
                .child(
                    div()
                        .flex_shrink_0()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(value),
                )
        };
        if let Some(snapshot) = shell.session.status_snapshot() {
            let page_status::TrayReadings {
                health,
                machine: identity,
                metrics,
            } = page_status::tray_readings(&t, locale, snapshot);
            let header = div()
                .flex()
                .flex_col()
                .gap(rems(0.2))
                .child(caption(copy::pick(locale, "Health evidence", "健康证据")))
                .child(
                    div()
                        .text_size(ui::TYPE_TITLE)
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(health),
                )
                .child(
                    div()
                        .text_size(ui::TYPE_META)
                        .text_color(rgb(t.muted))
                        .child(identity.join(" · ")),
                );
            body = body.child(header);
            let mut grid = div().flex().flex_col().gap(rems(0.25));
            for pair in metrics.chunks(2) {
                grid = grid.child(div().flex().flex_row().gap(rems(0.25)).children(
                    pair.iter().map(|(label, value)| {
                        div()
                            .flex_1()
                            .min_w_0()
                            .rounded(ui::RADIUS_ITEM)
                            .bg(t.well.color())
                            .px(ui::SPACE_1)
                            .py(rems(0.25))
                            .child(caption(label))
                            .child(div().truncate().child(value.clone()))
                    }),
                ));
            }
            let processes = crate::status_panel::sorted_processes(
                snapshot,
                "",
                crate::status_panel::ProcessSort::Cpu,
            );
            let mut list = div()
                .flex()
                .flex_col()
                .gap(rems(0.15))
                .child(caption(copy::pick(
                    locale,
                    "Top processes by CPU",
                    "CPU 占用最高的进程",
                )));
            if processes.is_empty() {
                list = list.child(ui::muted(
                    &t,
                    copy::pick(locale, "No process readings", "暂无进程读数"),
                ));
            }
            for process in processes.into_iter().take(5) {
                list = list.child(row(
                    process.name.clone(),
                    format!(
                        "{:.1}%",
                        process.cpu_basis_points_of_one_logical_core as f32 / 100.
                    ),
                ));
            }
            body = body.child(grid).child(list);
        } else {
            let label = if shell.session.status_live_fault().is_some() {
                copy::pick(locale, "Live readings unavailable", "实时读数不可用")
            } else {
                copy::pick(locale, "Waiting for readings…", "等待读数…")
            };
            body = body.child(ui::muted(&t, label));
        }
        let mut history = div()
            .flex()
            .flex_col()
            .gap(rems(0.15))
            .child(caption(copy::pick(locale, "Activity totals", "操作累计")));
        if let Some(Some(totals)) = self.totals {
            let minimum = |value: String, lower| {
                if lower {
                    format!("{} {value}", copy::pick(locale, "at least", "至少"))
                } else {
                    value
                }
            };
            history = history
                .child(row(
                    copy::pick(locale, "Moved to Recycle Bin", "已移入回收站").into(),
                    minimum(
                        ui::bytes(totals.clean.known_bytes),
                        totals.clean.lower_bound,
                    ),
                ))
                .child(row(
                    copy::pick(locale, "Apps uninstalled", "已卸载应用").into(),
                    minimum(
                        totals.software.succeeded.to_string(),
                        totals.software.lower_bound,
                    ),
                ))
                .child(row(
                    copy::pick(locale, "Maintenance completed", "已完成维护").into(),
                    minimum(
                        totals.optimize.succeeded.to_string(),
                        totals.optimize.lower_bound,
                    ),
                ));
            if totals.clean.unknown_records > 0 {
                history = history.child(
                    div()
                        .text_size(ui::TYPE_META)
                        .text_color(rgb(t.muted))
                        .child(format!(
                            "{} {}",
                            copy::pick(locale, "Moves with unknown size", "容量未知的移动记录"),
                            totals.clean.unknown_records
                        )),
                );
            }
        } else {
            history = history.child(ui::muted(
                &t,
                if self.totals.is_some() {
                    copy::pick(locale, "History unavailable", "历史不可用")
                } else {
                    copy::pick(locale, "Reading history…", "正在读取历史…")
                },
            ));
        }
        let awake = ui::button(
            &t,
            "tray-awake",
            copy::pick(locale, "Keep awake…", "保持唤醒…"),
            if shell.session.host_action_busy() {
                ButtonKind::Disabled
            } else {
                ButtonKind::Secondary
            },
        )
        .on_click(|_, _, cx| cx.global::<Runtime>().tray.emit(Event::Awake));
        div()
            .id("tray-panel")
            .track_focus(&self.focus)
            .on_action(|_: &Back, window, _| window.remove_window())
            .role(gpui::Role::Group)
            .aria_label(copy::pick(locale, "DevSweep status", "DevSweep 状态"))
            .flex()
            .flex_col()
            .size_full()
            .bg(t.sheet_solid.color())
            .text_color(rgb(t.ink))
            .text_size(ui::TYPE_BODY)
            .font_family(ui::FONT_FAMILY)
            .child(
                body.child(history)
                    .when(shell.session.keep_awake_available(), |node| {
                        node.child(awake)
                    }),
            )
    }
}

#[cfg(test)]
mod receipt;
