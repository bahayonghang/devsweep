//! Analyze page: root choice, then a treemap and an accessible list of the
//! current folder that share one node id and one selection. The recycle
//! preview binds a digest; only the explicit confirm button hands that digest
//! to the CLI's executor, which moves to the Recycle Bin and never deletes.

use super::*;
use crate::analyze::NodeFace;
use devsweep_core::analysis::{AnalyzeEvidence, AnalyzeNodeKind, AnalyzeNodeV1};

const TREEMAP_W: f32 = 600.0;
const TREEMAP_H: f32 = 300.0;

fn observed_size(locale: copy::Locale, bytes: u64, evidence: AnalyzeEvidence) -> String {
    match evidence {
        AnalyzeEvidence::Complete => ui::bytes(bytes),
        AnalyzeEvidence::Incomplete => match locale {
            copy::Locale::En => format!("At least {}", ui::bytes(bytes)),
            copy::Locale::ZhCn => format!("至少 {}", ui::bytes(bytes)),
        },
        AnalyzeEvidence::Unknown => copy::pick(locale, "Size unknown", "容量未知").to_string(),
    }
}

pub(super) fn menu_bindings() -> [KeyBinding; 9] {
    [
        KeyBinding::new("menu", AnalyzeContextMenu, Some("AnalyzeNode")),
        KeyBinding::new("shift-f10", AnalyzeContextMenu, Some("AnalyzeNode")),
        KeyBinding::new("escape", AnalyzeMenuClose, Some("AnalyzeMenu")),
        KeyBinding::new("down", AnalyzeMenuNext, Some("AnalyzeMenu")),
        KeyBinding::new("tab", AnalyzeMenuNext, Some("AnalyzeMenu")),
        KeyBinding::new("up", AnalyzeMenuPrevious, Some("AnalyzeMenu")),
        KeyBinding::new("shift-tab", AnalyzeMenuPrevious, Some("AnalyzeMenu")),
        KeyBinding::new("enter", AnalyzeMenuChoose, Some("AnalyzeMenu")),
        KeyBinding::new("space", AnalyzeMenuChoose, Some("AnalyzeMenu")),
    ]
}

/// The focused originating control remains alive while its in-window menu is open.
pub(super) struct NodeMenu {
    node: u32,
    tile: bool,
    origin: FocusHandle,
    pub(super) items: [FocusHandle; 3],
    position: Option<gpui::Point<gpui::Pixels>>,
    recycle_refusal: Option<String>,
}

impl ShellView {
    fn analyze_menu_target(
        &self,
        target: gpui::Stateful<gpui::Div>,
        t: &Theme,
        (id, tile): (u32, bool),
        host: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let focus = host
            .use_keyed_state(
                gpui::ElementId::named_usize(
                    if tile {
                        "analyze-tile-focus"
                    } else {
                        "analyze-row-focus"
                    },
                    id as usize,
                ),
                cx,
                |_, cx| cx.focus_handle().tab_stop(true),
            )
            .read(cx)
            .clone();
        let origin = focus.clone();
        let mut target = target
            .track_focus(&focus)
            .key_context("AnalyzeNode")
            .on_action(cx.listener(move |this, _: &AnalyzeContextMenu, host, cx| {
                cx.stop_propagation();
                this.open_analyze_menu(id, tile, origin.clone(), None, host, cx);
            }))
            .on_mouse_down(
                gpui::MouseButton::Right,
                cx.listener(move |this, event: &gpui::MouseDownEvent, host, cx| {
                    cx.stop_propagation();
                    // A focusable row normally takes focus on mouse-down. The
                    // popup now owns it, including during deferred painting.
                    host.prevent_default();
                    this.open_analyze_menu(id, tile, focus.clone(), Some(event.position), host, cx);
                }),
            );
        if let Some(menu) = self
            .analyze_menu
            .as_ref()
            .filter(|menu| menu.node == id && menu.tile == tile)
        {
            let mut anchor = gpui::anchored().snap_to_window();
            if let Some(position) = menu.position {
                anchor = anchor.position(position);
            }
            target = target.child(
                gpui::deferred(anchor.child(self.analyze_menu_items(t, menu, cx))).with_priority(5),
            );
        }
        target
    }

    fn open_analyze_menu(
        &mut self,
        id: u32,
        tile: bool,
        origin: FocusHandle,
        position: Option<gpui::Point<gpui::Pixels>>,
        host: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(window) = self.session.analyze_window() else {
            return;
        };
        let locale = self.session.locale();
        let recycle_refusal = if !window.board().eligible(id) || window.stale() {
            Some(
                copy::pick(
                    locale,
                    "Rescan or choose an eligible item",
                    "请重新扫描或选择符合条件的项目",
                )
                .to_string(),
            )
        } else {
            match devsweep_core::analysis::preview_analyze_trash(
                "analyze-menu",
                window.snapshot(),
                &[id],
            ) {
                Ok(preview) => preview.refused.first().map(|item| {
                    crate::analyze_window::refusal_label(locale, item.reason_code).to_string()
                }),
                Err(_) => Some(
                    copy::pick(locale, "Cannot verify this item", "无法核实此项目").to_string(),
                ),
            }
        };
        let items = std::array::from_fn(|_| cx.focus_handle().tab_stop(true));
        self.session.analyze_context_focus(id);
        host.focus(&items[0], cx);
        let first = items[0].clone();
        let view = cx.weak_entity();
        host.on_next_frame(move |host, cx| {
            if view.upgrade().is_some_and(|view| {
                view.read(cx)
                    .analyze_menu
                    .as_ref()
                    .is_some_and(|menu| menu.items[0] == first)
            }) {
                host.focus(&first, cx);
            }
        });
        self.analyze_menu = Some(NodeMenu {
            node: id,
            tile,
            origin,
            items,
            position,
            recycle_refusal,
        });
        cx.notify();
    }

    pub(super) fn close_analyze_menu(&mut self, host: &mut Window, cx: &mut Context<Self>) {
        if let Some(menu) = self.analyze_menu.take() {
            host.focus(&menu.origin, cx);
            cx.notify();
        }
    }

    pub(super) fn step_analyze_menu(&self, back: bool, host: &mut Window, cx: &mut Context<Self>) {
        if let Some(menu) = &self.analyze_menu {
            let mut i = menu
                .items
                .iter()
                .position(|item| item.is_focused(host))
                .unwrap_or(0);
            loop {
                i = (i + if back { 2 } else { 1 }) % 3;
                if i != 1 || menu.recycle_refusal.is_none() {
                    break;
                }
            }
            host.focus(&menu.items[i], cx);
        }
    }

    pub(super) fn choose_analyze_menu(
        &mut self,
        index: usize,
        host: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(menu) = &self.analyze_menu else {
            return;
        };
        let id = menu.node;
        if index == 1 && menu.recycle_refusal.is_some() {
            return;
        }
        self.close_analyze_menu(host, cx);
        match index {
            0 => {
                let _ = self.session.analyze_reveal();
            }
            1 => self.session.analyze_mark(id, false),
            _ => self.session.analyze_explain(id),
        }
        cx.notify();
    }

    fn analyze_menu_items(
        &self,
        t: &Theme,
        menu: &NodeMenu,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let locale = self.session.locale();
        let marked = self
            .session
            .analyze_window()
            .is_some_and(|w| w.board().is_checked(menu.node));
        let toggle = if marked {
            copy::pick(locale, "Remove from selection", "从选择中移除")
        } else {
            copy::pick(locale, "Add to Recycle Bin plan", "加入回收站计划")
        };
        let labels = [
            copy::pick(locale, "Show in Explorer", "在资源管理器中显示"),
            toggle,
            copy::pick(locale, "Explain item", "解释项目"),
        ];
        let mut popup = ui::overlay(t)
            .p(ui::SPACE_2)
            .gap(ui::SPACE_1)
            .id("analyze-node-menu")
            .accessibility_id("analyze-node-menu")
            .role(gpui::Role::Menu)
            .occlude()
            .key_context("AnalyzeMenu")
            .on_mouse_down_out(cx.listener(|this, _, host, cx| this.close_analyze_menu(host, cx)));
        for (index, label) in labels.into_iter().enumerate() {
            let enabled = index != 1 || menu.recycle_refusal.is_none();
            let item = ui::button(
                t,
                ["analyze-reveal", "analyze-menu-mark", "analyze-explain"][index],
                label,
                ButtonKind::Ghost,
            )
            .accessibility_id(["analyze-reveal", "analyze-menu-mark", "analyze-explain"][index])
            .role(gpui::Role::MenuItem)
            .track_focus(&menu.items[index])
            .when(enabled, |item| {
                item.on_click(cx.listener(move |this, _, host, cx| {
                    cx.stop_propagation();
                    this.choose_analyze_menu(index, host, cx);
                }))
            })
            .when(!enabled, |item| {
                item.tab_stop(false)
                    .opacity(0.5)
                    .a11y_synthetic_children(|tree| tree.parent_node().set_disabled())
            });
            popup = popup.child(item);
        }
        popup
            .when_some(menu.recycle_refusal.as_ref(), |panel, reason| {
                panel.child(ui::muted(t, reason.clone()))
            })
            .into_any_element()
    }

    pub(super) fn analyze_page(
        &mut self,
        t: &Theme,
        host: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let locale = self.session.locale();
        let detail = self.session.analyze_window().is_some();
        let mut actions = vec![ui::any(
            ui::icon_button(
                t,
                "analyze-choose-root",
                "folder",
                copy::clean_root_choose(locale),
                if detail || self.session.analyze_roots().is_empty() {
                    ButtonKind::Primary
                } else {
                    ButtonKind::Secondary
                },
            )
            .on_click(cx.listener(|this, _, _, cx| this.analyze_pick_root(cx))),
        )];
        if detail {
            actions.insert(
                0,
                ui::any(
                    ui::icon_button(
                        t,
                        "analyze-back",
                        "chevron-left",
                        copy::back_label(locale),
                        ButtonKind::Ghost,
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.session.close_mode_detail();
                        cx.notify();
                    })),
                ),
            );
        }
        // The open folder is named once, here; the detail row below keeps
        // only the up button and the size and completeness tags.
        let status = Some(match self.session.analyze_window() {
            Some(window) => ui::display_path_text(&window.breadcrumb()),
            None => copy::analyze_locations_status(locale, self.session.analyze_roots().len()),
        });
        let bar = page_bar(t, status, None, actions);
        let mut page = div()
            .flex()
            .flex_col()
            .gap(ui::SPACE_4)
            .children(self.shared_banners(t));
        if let Some(notice) = self.session.analyze_notice() {
            page = page.child(ui::banner(
                t,
                Tone::Warning,
                crate::analyze_window::localized_error(locale, notice),
            ));
        }
        if let Some(path) = self.session.analyze_scanning() {
            let label = path.display().to_string();
            let shown = ui::display_path(path);
            page = page.child(
                ui::card(t)
                    .id("analyze-scanning")
                    .role(gpui::Role::Status)
                    .aria_label(SharedString::from(label.clone()))
                    .gap(ui::SPACE_3)
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(ui::SPACE_3)
                            .child(ui::icon_tile(t, "search", t.input))
                            .child(ui::two_line(
                                t,
                                copy::pick(locale, "Mapping folder…", "正在分析文件夹…"),
                                shown,
                            ))
                            .child(
                                ui::icon_button(
                                    t,
                                    "analyze-cancel",
                                    "stop",
                                    copy::clean_cancel_action(locale),
                                    ButtonKind::Secondary,
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.session.analyze_cancel_scan();
                                    cx.notify();
                                })),
                            ),
                    )
                    .child(ui::progress_track(t))
                    .child(ui::muted(
                        t,
                        copy::pick(
                            locale,
                            "Read-only walk. Large drives take a while; the window stays responsive.",
                            "只读遍历。大磁盘需要一些时间，窗口保持可用。",
                        ),
                    )),
            );
        }
        page = if detail {
            page.child(self.analyze_detail(t, host, cx))
        } else {
            page.child(self.analyze_landing(t, cx))
        };
        let footer = detail.then(|| self.analyze_footer(t, cx));
        sheet_frame(bar, page.into_any_element(), footer)
    }

    fn analyze_landing(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let roots = self.session.analyze_roots();
        if roots.is_empty() {
            return ui::any(ui::card(t).child(ui::empty_state(
                t,
                "drive",
                copy::analyze_empty_title(locale),
                copy::analyze_empty_detail(locale),
            )));
        }
        let cards = roots.iter().enumerate().map(|(index, choice)| {
            let ready = choice.error.is_none();
            let label = choice
                .normalized
                .clone()
                .unwrap_or_else(|| choice.input.clone());
            let hover = t.input;
            let hover_fill = t.control_fill.color();
            let high_contrast = t.high_contrast;
            div()
                .id(ElementId::Name(SharedString::from(format!(
                    "analyze-root-{index}"
                ))))
                .role(gpui::Role::Button)
                .accessibility_id(SharedString::from(format!("analyze-root-{index}")))
                .aria_label(SharedString::from(format!(
                    "{label} {}",
                    choice
                        .error
                        .as_deref()
                        .map(|err| crate::analyze_window::localized_error(locale, err))
                        .unwrap_or_else(|| copy::pick(locale, "Ready", "可用").to_string())
                )))
                .flex()
                .flex_row()
                .items_center()
                .gap(ui::SPACE_2)
                .w(rems(20.0))
                .max_w_full()
                .p(ui::SPACE_3)
                .rounded(ui::RADIUS_WELL)
                // A filled well; fill carries the boundary. High contrast
                // keeps the 1px border.
                .bg(t.well.color())
                .when(t.high_contrast, |c| c.border_1().border_color(rgb(t.ink)))
                .when(ready, |c| {
                    c.cursor_pointer()
                        .hover(move |s| {
                            if high_contrast {
                                s.border_color(rgb(hover))
                            } else {
                                s.bg(hover_fill)
                            }
                        })
                        .keyboard_focus(t)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let _ = this.session.analyze_open_listed(index);
                            cx.notify();
                        }))
                })
                .when(!ready, |c| {
                    c.a11y_synthetic_children(|tree| tree.parent_node().set_disabled())
                })
                .child(ui::icon_tile(
                    t,
                    "drive",
                    if ready { t.input } else { t.muted },
                ))
                .child(ui::two_line(
                    t,
                    ui::display_path_text(&label),
                    if ready {
                        match choice.capacity {
                            Some(devsweep_core::analysis::AnalyzeVolumeCapacityV1::Available {
                                total_bytes,
                                available_bytes,
                            }) => match locale {
                                copy::Locale::En => format!(
                                    "{} available of {}",
                                    ui::bytes(available_bytes),
                                    ui::bytes(total_bytes)
                                ),
                                copy::Locale::ZhCn => format!(
                                    "共 {}，可用 {}",
                                    ui::bytes(total_bytes),
                                    ui::bytes(available_bytes)
                                ),
                            },
                            _ => copy::pick(locale, "Capacity unavailable", "容量未知").to_string(),
                        }
                    } else {
                        crate::analyze_window::localized_error(
                            locale,
                            choice.error.as_deref().unwrap_or_default(),
                        )
                    },
                ))
                .when_some(
                    choice.capacity.and_then(|capacity| match capacity {
                        devsweep_core::analysis::AnalyzeVolumeCapacityV1::Available {
                            total_bytes,
                            available_bytes,
                        } if total_bytes > 0 => {
                            Some(1.0 - available_bytes as f32 / total_bytes as f32)
                        }
                        _ => None,
                    }),
                    |card, used| card.child(div().w(rems(3.)).child(ui::meter(t, used, t.input))),
                )
                .when(ready, |c| c.child(ui::icon("chevron-right", t.muted)))
        });
        div()
            .flex()
            .flex_col()
            .gap(ui::SPACE_2)
            .child(ui::eyebrow(t, copy::pick(locale, "LOCATIONS", "位置")))
            .child(
                div()
                    .id("analyze-actions")
                    .role(gpui::Role::Group)
                    .aria_label(copy::mode_name(locale, "analyze"))
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap(ui::SPACE_3)
                    .children(cards),
            )
            .into_any_element()
    }

    fn analyze_detail(
        &self,
        t: &Theme,
        host: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let locale = self.session.locale();
        let Some(window) = self.session.analyze_window() else {
            return div().into_any_element();
        };
        let snapshot = window.snapshot();
        let node = |id: u32| window.board().node(id);
        let parent_id = window.parent_id();
        let parent_bytes = node(parent_id).map(|n| n.bytes).unwrap_or(0);
        let selected = window.board().list_selection();
        let can_go_up = self.session.analyze_can_go_up();
        let largest = window.largest_active();
        let ranked = window.largest();
        let children: Vec<&AnalyzeNodeV1> =
            window.child_ids().into_iter().filter_map(node).collect();
        let wide = host.viewport_size().width >= host.rem_size() * 57.;
        // Treemap of the current folder.
        let items = children
            .iter()
            .take(60)
            .map(|n| (n.bytes.max(1), n.id.to_string()))
            .collect::<Vec<_>>();
        let rects = clean_map::squarify_items(
            &items,
            MapBounds {
                width: TREEMAP_W,
                height: TREEMAP_H,
            },
        );
        let mut canvas = div()
            .id("analyze-treemap")
            .role(gpui::Role::Group)
            .aria_label(copy::analyze_treemap_title(locale))
            .relative()
            .w_full()
            .h(px(TREEMAP_H))
            .rounded(ui::RADIUS_ITEM)
            .overflow_hidden()
            .bg(t.control_fill.color());
        // Tile labels are rem sized, so their fit thresholds follow the text
        // scale while the map geometry stays in pixels.
        let text_scale = f32::from(self.session.text_scale()) / 100.0;
        for rect in rects {
            let child = children[rect.index];
            let id = child.id;
            let is_selected = window.board().is_checked(id);
            let size = observed_size(locale, child.bytes, child.evidence);
            let hue = match child.kind {
                AnalyzeNodeKind::Directory | AnalyzeNodeKind::File => {
                    t.category([0, 2, 1, 4, 6, 3][rect.index % 6])
                }
                AnalyzeNodeKind::Reparse => t.category(5),
            };
            let (w, h) = ((rect.width - 2.0).max(1.0), (rect.height - 2.0).max(1.0));
            let mut tile = div()
                .id(gpui::ElementId::named_usize("analyze-tile", id as usize))
                .accessibility_id(SharedString::from(format!("analyze-tile-{id}")))
                .role(gpui::Role::Button)
                .aria_label(SharedString::from(format!("{} {}", child.name, size)))
                .absolute()
                .left(gpui::relative((rect.x + 1.0) / TREEMAP_W))
                .top(px(rect.y + 1.0))
                .w(gpui::relative(w / TREEMAP_W))
                .h(px(h))
                .rounded(ui::RADIUS_CHECK)
                .overflow_hidden()
                // Horizontal padding only: the smallest spacing step on all
                // four sides would leave a short tile no line height.
                .px(ui::SPACE_1)
                .bg(t.tint(hue, if is_selected { 0x30 } else { 0x18 }))
                .border_1()
                .border_color(if is_selected { rgb(t.ink) } else { rgb(hue) })
                .cursor_pointer()
                .hover(|s| s.border_color(rgb(t.ink)))
                .keyboard_focus(t)
                .on_click(cx.listener(move |this, event: &gpui::ClickEvent, _, cx| {
                    this.analyze_node_click(id, event, false, cx);
                }));
            tile = self.analyze_menu_target(tile, t, (id, true), host, cx);
            if w >= 54.0 * text_scale && h >= 22.0 * text_scale {
                tile = tile.child(
                    div()
                        .truncate()
                        .text_size(ui::TYPE_META)
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(t.ink))
                        .child(SharedString::from(child.name.clone())),
                );
                if h >= 38.0 * text_scale {
                    tile = tile.child(
                        div()
                            .text_size(ui::TYPE_META)
                            .text_color(rgb(t.ink))
                            .child(SharedString::from(size)),
                    );
                }
            }
            canvas = canvas.child(tile);
        }
        if children.is_empty() {
            canvas = canvas.child(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(ui::muted(t, copy::analyze_face_label(locale, "empty"))),
            );
        }

        // Size-ranked list.
        let mut list = div()
            .id("analyze-list")
            .role(gpui::Role::List)
            .aria_label(copy::analyze_list_title(locale))
            .max_h(rems(24.0))
            .overflow_y_scroll()
            .flex()
            .flex_col();
        for (index, child) in children.iter().take(200).enumerate() {
            let id = child.id;
            let size = observed_size(locale, child.bytes, child.evidence);
            let name = if largest {
                ui::display_path_text(&ranked.files[index].path)
            } else {
                child.name.clone()
            };
            let face = window.board().face(id).unwrap_or(NodeFace::Present);
            let is_selected = window.board().is_checked(id);
            let share = child.bytes as f32 / parent_bytes.max(1) as f32;
            let icon = match child.kind {
                AnalyzeNodeKind::Directory => "folder",
                AnalyzeNodeKind::File => "rules",
                AnalyzeNodeKind::Reparse => "external",
            };
            let element_id = SharedString::from(format!("analyze-row-{id}"));
            let face_badge = match face {
                NodeFace::Present => None,
                NodeFace::Empty => Some((copy::analyze_face_label(locale, "empty"), Tone::Neutral)),
                NodeFace::Unreadable => Some((
                    copy::analyze_face_label(locale, "unreadable"),
                    Tone::Warning,
                )),
            };
            let drillable = child.kind == AnalyzeNodeKind::Directory;
            let row = ui::list_row(
                t,
                ElementId::Name(element_id.clone()),
                is_selected,
                selected == Some(id),
            )
            .accessibility_id(element_id)
            .aria_label(SharedString::from(format!(
                "{} {} {}",
                name,
                size,
                copy::analyze_face_label(locale, face.as_str())
            )))
            .py(ui::SPACE_1)
            .on_click(cx.listener(move |this, event: &gpui::ClickEvent, _, cx| {
                this.analyze_node_click(id, event, drillable, cx);
            }))
            .child(
                ui::checkbox(t, is_selected, window.board().eligible(id))
                    .id(gpui::ElementId::named_usize("analyze-check", id as usize))
                    .accessibility_id(SharedString::from(format!("analyze-check-{id}")))
                    .role(gpui::Role::CheckBox)
                    .aria_label(SharedString::from(name.clone()))
                    .aria_toggled(if is_selected {
                        gpui::Toggled::True
                    } else {
                        gpui::Toggled::False
                    })
                    .when(window.board().eligible(id), |check| {
                        check
                            .keyboard_focus(t)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.session.analyze_mark(id, false);
                                cx.notify();
                            }))
                    })
                    .when(!window.board().eligible(id), |check| {
                        check.a11y_synthetic_children(|tree| tree.parent_node().set_disabled())
                    }),
            )
            .child(ui::icon(icon, if drillable { t.input } else { t.muted }))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(ui::SPACE_1)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(ui::TYPE_BODY)
                                    .child(SharedString::from(name)),
                            )
                            .when_some(face_badge, |d, (label, tone)| {
                                d.child(ui::badge(t, label, tone))
                            })
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .text_size(ui::TYPE_BODY)
                                    .font_features(ui::tabular_numerals())
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(SharedString::from(size)),
                            ),
                    )
                    .child(ui::meter(t, share, t.input)),
            )
            .when(drillable, |row| {
                row.child(ui::icon("chevron-right", t.muted))
            });
            list = list.child(self.analyze_menu_target(row, t, (id, false), host, cx));
        }

        let detail = selected.and_then(node).map(|chosen| {
            let mut card =
                ui::card(t)
                    .id("analyze-selection")
                    .gap(ui::SPACE_1)
                    .child(ui::two_line(
                        t,
                        ui::display_path_text(&chosen.name),
                        observed_size(locale, chosen.bytes, chosen.evidence),
                    ));
            if let Some(result) = window.explanation(chosen.id) {
                card = card.child(match result {
                    Ok(explained) => {
                        let class =
                            copy::analyze_class_label(locale, &format!("{:?}", explained.class));
                        let owner_count = explained.lock.owners.len();
                        let label = match locale {
                            copy::Locale::En => {
                                format!("{class} · {owner_count} observed lock owners")
                            }
                            copy::Locale::ZhCn => {
                                format!("{class} · 检测到 {owner_count} 个占用进程")
                            }
                        };
                        ui::muted(
                            t,
                            if explained.lock.supported {
                                label
                            } else {
                                class.to_string()
                            },
                        )
                    }
                    Err(error) => {
                        ui::muted(t, crate::analyze_window::localized_error(locale, error))
                    }
                });
            }
            card
        });
        let list_card = ui::card(t)
            .min_w_0()
            .gap(ui::SPACE_1)
            .when(wide, |card| card.w(gpui::relative(0.40)).flex_shrink_0())
            .when(!wide, |card| card.w_full())
            .child(ui::card_header(
                t,
                if largest {
                    copy::pick(locale, "Largest files", "最大文件")
                } else {
                    copy::analyze_list_title(locale)
                },
            ))
            .when(largest, |card| {
                card.child(ui::muted(
                    t,
                    match locale {
                        copy::Locale::En => format!(
                            "{} of {} retained files at least {}.",
                            ranked.files.len(),
                            ranked.matching_files,
                            ui::bytes(ranked.min_bytes)
                        ),
                        copy::Locale::ZhCn => format!(
                            "仅限快照中保留且至少为 {} 的文件，显示 {} / {} 项。",
                            ui::bytes(ranked.min_bytes),
                            ranked.files.len(),
                            ranked.matching_files
                        ),
                    },
                ))
            })
            .when(largest && children.is_empty(), |card| {
                card.child(ui::muted(
                    t,
                    copy::pick(locale, "No matching files", "没有符合条件的文件"),
                ))
            })
            .child(list);
        let map_card = ui::card(t)
            .min_w_0()
            .gap(ui::SPACE_2)
            .when(wide, |card| card.flex_1())
            .when(!wide, |card| card.w_full())
            .child(ui::card_header(t, copy::analyze_treemap_title(locale)))
            .child(canvas)
            .child(ui::muted(t, copy::analyze_treemap_description(locale)))
            .children(detail);
        let complete = snapshot.completeness
            == devsweep_core::analysis::AnalyzeCompleteness::Complete
            && snapshot
                .root_node()
                .is_some_and(|n| n.evidence == devsweep_core::analysis::AnalyzeEvidence::Complete);
        div()
            .flex()
            .flex_col()
            .gap(ui::SPACE_3)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .flex_wrap()
                    .gap(ui::SPACE_2)
                    .children(
                        [
                            (false, copy::pick(locale, "Folder", "文件夹")),
                            (true, copy::pick(locale, "Largest files", "最大文件")),
                        ]
                        .into_iter()
                        .map(|(show, label)| {
                            ui::button(
                                t,
                                if show {
                                    "analyze-largest"
                                } else {
                                    "analyze-folder"
                                },
                                label,
                                if show == largest {
                                    ButtonKind::Secondary
                                } else {
                                    ButtonKind::Ghost
                                },
                            )
                            .accessibility_id(if show {
                                "analyze-largest"
                            } else {
                                "analyze-folder"
                            })
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.session.analyze_show_largest(show);
                                    cx.notify();
                                },
                            ))
                        }),
                    )
                    .when(can_go_up && !largest, |row| {
                        row.child(
                            ui::icon_button(
                                t,
                                "analyze-up",
                                "chevron-left",
                                copy::pick(locale, "Up", "上一级"),
                                ButtonKind::Ghost,
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.session.analyze_go_up();
                                cx.notify();
                            })),
                        )
                    })
                    .child(ui::badge(
                        t,
                        if complete {
                            copy::pick(locale, "Scan complete", "扫描完成")
                        } else {
                            copy::pick(locale, "Partial snapshot", "快照不完整")
                        },
                        Tone::Neutral,
                    )),
            )
            .child(
                div()
                    .flex()
                    .when(wide, |d| d.flex_row())
                    .when(!wide, |d| d.flex_col())
                    .gap(ui::SPACE_3)
                    .child(list_card)
                    .child(map_card),
            )
            .into_any_element()
    }

    fn analyze_node_click(
        &mut self,
        id: u32,
        event: &gpui::ClickEvent,
        drillable: bool,
        cx: &mut Context<Self>,
    ) {
        let mods = event.modifiers();
        if mods.control || mods.shift {
            self.session.analyze_mark(id, mods.shift);
        } else if drillable {
            self.session
                .activate_mode_row(Mode::Analyze, id.to_string());
        } else {
            self.session.analyze_select(id);
        }
        cx.notify();
    }

    fn analyze_footer(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let window = self.session.analyze_window().expect("detail");
        let count = window.board().checked_count();
        let bytes = ui::bytes(window.board().selected_bytes());
        let label = match locale {
            copy::Locale::En => format!("{count} selected · {bytes}"),
            copy::Locale::ZhCn => format!("已选 {count} 项 · {bytes}"),
        };
        let armed = window.recycle_digest().is_some() && !window.stale();
        ui::card(t)
            .gap(ui::SPACE_1)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .flex_wrap()
                    .gap(ui::SPACE_2)
                    .child(div().flex_1().child(label))
                    .when(count > 0 && !window.stale() && !armed, |bar| {
                        bar.child(
                            ui::button(
                                t,
                                "analyze-preview-recycle",
                                copy::pick(locale, "Preview Recycle Bin move", "预览移到回收站"),
                                ButtonKind::Secondary,
                            )
                            .accessibility_id("analyze-preview-recycle")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.session.analyze_preview_recycle();
                                cx.notify();
                            })),
                        )
                    })
                    .when(armed, |bar| {
                        bar.child(
                            ui::button(
                                t,
                                "analyze-confirm-recycle",
                                copy::pick(locale, "Move to Recycle Bin", "移到回收站"),
                                ButtonKind::Danger,
                            )
                            .accessibility_id("analyze-confirm-recycle")
                            .on_click(cx.listener(|this, _, _, cx| {
                                let _ = this.session.analyze_confirm_recycle();
                                cx.notify();
                            })),
                        )
                        .child(
                            ui::button(
                                t,
                                "analyze-cancel-recycle",
                                copy::clean_cancel_action(locale),
                                ButtonKind::Ghost,
                            )
                            .accessibility_id("analyze-cancel-recycle")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.session.analyze_cancel_recycle();
                                cx.notify();
                            })),
                        )
                    }),
            )
            .when_some(window.recycle_line(), |bar, line| {
                bar.child(ui::muted(t, line.to_string()))
            })
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observed_sizes_distinguish_zero_unknown_and_lower_bound_in_both_languages() {
        for (locale, unknown, minimum) in [
            (copy::Locale::En, "Size unknown", "At least"),
            (copy::Locale::ZhCn, "容量未知", "至少"),
        ] {
            assert_eq!(observed_size(locale, 0, AnalyzeEvidence::Unknown), unknown);
            assert_eq!(
                observed_size(locale, 0, AnalyzeEvidence::Complete),
                ui::bytes(0)
            );
            assert_eq!(
                observed_size(locale, 42, AnalyzeEvidence::Incomplete),
                format!("{minimum} {}", ui::bytes(42))
            );
        }
    }

    #[test]
    fn analyze_menu_shortcuts_win_over_shell_f10_and_escape() {
        let mut keymap = gpui::Keymap::new(vec![
            KeyBinding::new("f10", MenuToggle, None),
            KeyBinding::new("escape", Back, None),
        ]);
        keymap.add_bindings(menu_bindings());
        let action = |context: &str, key: &str| {
            let (bindings, pending) = keymap.bindings_for_input(
                &[gpui::Keystroke::parse(key).unwrap()],
                &[gpui::KeyContext::parse(context).unwrap()],
            );
            assert!(!pending);
            bindings[0].action().name().to_string()
        };
        for key in ["menu", "shift-f10"] {
            assert_eq!(action("AnalyzeNode", key), AnalyzeContextMenu.name());
        }
        assert_eq!(
            action("ShellRoot AnalyzeMenu", "escape"),
            AnalyzeMenuClose.name()
        );
        assert_eq!(action("ShellRoot", "escape"), Back.name());
        assert_eq!(action("ShellRoot", "f10"), MenuToggle.name());
    }
}
