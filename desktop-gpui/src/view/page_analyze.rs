//! Analyze page: root choice, then a treemap and an accessible list of the
//! current folder that share one node id and one selection. Read-only; the
//! recycle preview binds a digest and never moves anything by itself.

use super::*;
use crate::analyze::NodeFace;
use devsweep_core::analysis::{AnalyzeNodeKind, AnalyzeNodeV1};

const TREEMAP_W: f32 = 600.0;
const TREEMAP_H: f32 = 300.0;

impl ShellView {
    pub(super) fn analyze_page(&mut self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
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
        let mut page = div()
            .flex()
            .flex_col()
            .gap(rems(1.25))
            .child(self.page_header(
                t,
                "analyze",
                copy::mode_name(locale, "analyze"),
                copy::mode_subtitle(locale, "analyze"),
                actions,
            ))
            .children(self.shared_banners(t));
        if let Some(notice) = self.session.analyze_notice() {
            page = page.child(ui::banner(t, Tone::Warning, notice.to_string()));
        }
        page = if detail {
            page.child(self.analyze_detail(t, cx))
        } else {
            page.child(self.analyze_landing(t, cx))
        };
        page.into_any_element()
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
            let hover = t.accent;
            div()
                .id(ElementId::Name(SharedString::from(format!(
                    "analyze-root-{index}"
                ))))
                .role(gpui::Role::Button)
                .aria_label(SharedString::from(format!(
                    "{label} {}",
                    choice.error.as_deref().unwrap_or("ready")
                )))
                .flex()
                .flex_row()
                .items_center()
                .gap(rems(0.85))
                .w(rems(20.0))
                .p(rems(1.0))
                .rounded(rems(0.9))
                .bg(rgb(t.surface))
                .border_1()
                .border_color(rgb(t.border))
                .when(ready, |c| {
                    c.cursor_pointer()
                        .hover(move |s| s.border_color(rgb(hover)))
                })
                .when(!ready, |c| c.opacity(0.7))
                .on_click(cx.listener(move |this, _, _, cx| {
                    let _ = this.session.analyze_open_listed(index);
                    cx.notify();
                }))
                .child(ui::icon_tile(
                    t,
                    "drive",
                    if ready { t.accent } else { t.text_subtle },
                ))
                .child(ui::two_line(
                    t,
                    label,
                    if ready {
                        copy::pick(locale, "Ready to map", "可以分析").to_string()
                    } else {
                        choice.error.clone().unwrap_or_default()
                    },
                ))
                .child(ui::icon("chevron-right", t.text_subtle))
        });
        div()
            .flex()
            .flex_col()
            .gap(rems(0.75))
            .child(ui::eyebrow(t, copy::pick(locale, "LOCATIONS", "位置")))
            .child(
                div()
                    .id("analyze-actions")
                    .role(gpui::Role::Group)
                    .aria_label(copy::mode_name(locale, "analyze"))
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap(rems(0.9))
                    .children(cards),
            )
            .into_any_element()
    }

    fn analyze_detail(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let Some(window) = self.session.analyze_window() else {
            return div().into_any_element();
        };
        let snapshot = window.snapshot();
        let node = |id: u32| snapshot.nodes.iter().find(|node| node.id == id);
        let parent_id = window.parent_id();
        let parent_bytes = node(parent_id).map(|n| n.bytes).unwrap_or(0).max(1);
        let selected = window.board().list_selection();
        let mut children: Vec<&AnalyzeNodeV1> = window
            .child_ids()
            .into_iter()
            .filter_map(&node)
            .filter(|n| n.id != parent_id)
            .collect();
        children.sort_by(|a, b| b.bytes.cmp(&a.bytes).then(a.name.cmp(&b.name)));

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
            .flex_shrink_0()
            .w(px(TREEMAP_W))
            .h(px(TREEMAP_H))
            .rounded(rems(0.6))
            .overflow_hidden()
            .bg(rgb(t.surface_alt));
        for rect in rects {
            let child = children[rect.index];
            let id = child.id;
            let is_selected = selected == Some(id);
            let hue = match child.kind {
                AnalyzeNodeKind::Directory | AnalyzeNodeKind::File => {
                    t.category([0, 2, 1, 4, 6, 3][rect.index % 6])
                }
                AnalyzeNodeKind::Reparse => t.category(5),
            };
            let (w, h) = ((rect.width - 2.0).max(1.0), (rect.height - 2.0).max(1.0));
            let mut tile = div()
                .id(gpui::ElementId::named_usize("analyze-tile", id as usize))
                .role(gpui::Role::Button)
                .aria_label(SharedString::from(format!(
                    "{} {}",
                    child.name,
                    ui::bytes(child.bytes)
                )))
                .absolute()
                .left(px(rect.x + 1.0))
                .top(px(rect.y + 1.0))
                .w(px(w))
                .h(px(h))
                .rounded(px(5.0))
                .overflow_hidden()
                .px(px(6.0))
                .py(px(4.0))
                .bg(t.tint(hue, if is_selected { 0x90 } else { 0x3A }))
                .border_1()
                .border_color(if is_selected {
                    rgb(t.text)
                } else {
                    t.tint(hue, 0x90)
                })
                .cursor_pointer()
                .hover(|s| s.opacity(0.85))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.session.analyze_select(id);
                    cx.notify();
                }));
            if w >= 54.0 && h >= 22.0 {
                tile = tile.child(
                    div()
                        .truncate()
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(t.text))
                        .child(SharedString::from(child.name.clone())),
                );
                if h >= 38.0 {
                    tile = tile.child(
                        div()
                            .text_size(px(10.5))
                            .text_color(rgb(t.text_muted))
                            .child(SharedString::from(ui::bytes(child.bytes))),
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
            .flex()
            .flex_col()
            .gap(rems(0.1));
        for child in children.iter().take(200) {
            let id = child.id;
            let face = window.board().face(id).unwrap_or(NodeFace::Present);
            let is_selected = selected == Some(id);
            let share = child.bytes as f32 / parent_bytes as f32;
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
            list = list.child(
                ui::list_row(
                    t,
                    ElementId::Name(element_id.clone()),
                    is_selected,
                    is_selected,
                )
                .accessibility_id(element_id)
                .aria_label(SharedString::from(format!(
                    "{} {} {}",
                    child.name,
                    ui::bytes(child.bytes),
                    face.as_str()
                )))
                .py(rems(0.45))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if drillable {
                        this.session
                            .activate_mode_row(Mode::Analyze, id.to_string());
                    } else {
                        this.session.analyze_select(id);
                    }
                    cx.notify();
                }))
                .child(ui::icon(
                    icon,
                    if drillable { t.accent } else { t.text_muted },
                ))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .gap(rems(0.25))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(rems(0.5))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(rems(0.88))
                                        .child(SharedString::from(child.name.clone())),
                                )
                                .when_some(face_badge, |d, (label, tone)| {
                                    d.child(ui::badge(t, label, tone))
                                })
                                .child(
                                    div()
                                        .flex_shrink_0()
                                        .text_size(rems(0.82))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(SharedString::from(ui::bytes(child.bytes))),
                                ),
                        )
                        .child(ui::meter(t, share, t.accent)),
                )
                .when(drillable, |row| {
                    row.child(ui::icon("chevron-right", t.text_subtle))
                }),
            );
        }

        // Selection detail.
        let detail = selected.and_then(node).map(|chosen| {
            let face = window.board().face(chosen.id).unwrap_or(NodeFace::Present);
            let file = chosen.kind == AnalyzeNodeKind::File;
            let chosen_id = chosen.id;
            ui::card(t)
                .id("analyze-selection")
                .gap(rems(0.6))
                .child(ui::eyebrow(t, copy::pick(locale, "SELECTED", "已选择")))
                .child(
                    div()
                        .text_size(rems(1.05))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(SharedString::from(chosen.name.clone())),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .gap(rems(0.4))
                        .child(ui::badge(t, ui::bytes(chosen.bytes), Tone::Accent))
                        .child(ui::badge(
                            t,
                            copy::analyze_face_label(locale, face.as_str()),
                            Tone::Neutral,
                        ))
                        .child(ui::badge(
                            t,
                            match locale {
                                copy::Locale::En => format!("{} items", chosen.recursive_count),
                                copy::Locale::ZhCn => format!("{} 项", chosen.recursive_count),
                            },
                            Tone::Neutral,
                        )),
                )
                .child(ui::muted(t, window.explain_line(chosen_id)))
                .when_some(window.recycle_line(), |c, line| {
                    c.child(ui::banner(t, Tone::Info, line.to_string()))
                })
                .when(file, |c| {
                    c.child(
                        div().flex().flex_row().gap(rems(0.5)).child(
                            ui::icon_button(
                                t,
                                "analyze-preview-recycle",
                                "eye",
                                copy::pick(locale, "Preview Recycle Bin move", "预览移到回收站"),
                                ButtonKind::Secondary,
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.session
                                        .activate_mode_row(Mode::Analyze, chosen_id.to_string());
                                    cx.notify();
                                },
                            )),
                        ),
                    )
                })
        });

        let completeness = format!("{:?}", snapshot.completeness).to_ascii_lowercase();
        div()
            .flex()
            .flex_col()
            .gap(rems(1.0))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(rems(0.5))
                    .child(ui::icon("folder", t.text_muted))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(rems(0.88))
                            .text_color(rgb(t.text_muted))
                            .child(SharedString::from(window.breadcrumb())),
                    )
                    .child(ui::badge(t, ui::bytes(parent_bytes), Tone::Accent))
                    .child(ui::badge(t, completeness, Tone::Neutral)),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_start()
                    .gap(rems(1.0))
                    .child(
                        ui::card(t)
                            .gap(rems(0.75))
                            .child(ui::card_header(t, copy::analyze_treemap_title(locale)))
                            .child(canvas)
                            .child(ui::muted(t, copy::analyze_treemap_description(locale))),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w(rems(18.0))
                            .gap(rems(1.0))
                            .child(detail.unwrap_or_else(|| {
                                ui::card(t).id("analyze-selection").child(ui::empty_state(
                                    t,
                                    "eye",
                                    copy::pick(locale, "Nothing selected", "未选择"),
                                    copy::pick(
                                        locale,
                                        "Pick a tile or a row to see its size, evidence, and what it would take to move it to the Recycle Bin.",
                                        "选择一个色块或一行，查看大小、证据，以及移到回收站需要什么。",
                                    ),
                                ))
                            })),
                    ),
            )
            .child(
                ui::card(t)
                    .gap(rems(0.6))
                    .child(ui::card_header(t, copy::analyze_list_title(locale)))
                    .child(list),
            )
            .into_any_element()
    }
}
