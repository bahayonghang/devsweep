//! Settings and the supporting destinations opened from the beaver menu.

use super::*;
use crate::preferences::DesktopMotion;

impl ShellView {
    fn surface_back_button(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        ui::any(
            ui::icon_button(
                t,
                "surface-back",
                "chevron-left",
                copy::back_label(locale),
                ButtonKind::Ghost,
            )
            .on_click(cx.listener(|this, _, _, cx| {
                this.session.apply_key(crate::shell::NavKey::Escape);
                cx.notify();
            })),
        )
    }

    fn setting_row(
        &self,
        t: &Theme,
        title: impl Into<SharedString>,
        detail: impl Into<SharedString>,
        control: impl IntoElement,
    ) -> gpui::AnyElement {
        div()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap(rems(1.0))
            .py(rems(0.85))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w(rems(14.0))
                    .gap(rems(0.15))
                    .child(div().font_weight(FontWeight::MEDIUM).child(title.into()))
                    .child(ui::muted(t, detail)),
            )
            .child(control)
            .into_any_element()
    }

    pub(super) fn settings_page(&mut self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let theme = self.session.theme();
        let dark = matches!(
            Theme::for_preferences(theme, false),
            ui::Theme { dark: true, .. }
        );
        let language = ui::segmented(t)
            .id("settings-language")
            .role(gpui::Role::TabList)
            .aria_label(copy::language_label(locale))
            .child(
                ui::segment(
                    t,
                    "settings-language-en",
                    "English",
                    locale == copy::Locale::En,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.session.set_locale(copy::Locale::En);
                    cx.notify();
                })),
            )
            .child(
                ui::segment(
                    t,
                    "settings-language-zh",
                    "简体中文",
                    locale == copy::Locale::ZhCn,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.session.set_locale(copy::Locale::ZhCn);
                    cx.notify();
                })),
            );
        let appearance = ui::segmented(t)
            .id("settings-theme")
            .role(gpui::Role::TabList)
            .aria_label(copy::theme_label(locale))
            .child(
                ui::segment(
                    t,
                    "settings-theme-dark",
                    copy::pick(locale, "Dark", "深色"),
                    dark,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.session.set_theme(DesktopTheme::Dark);
                    cx.notify();
                })),
            )
            .child(
                ui::segment(
                    t,
                    "settings-theme-light",
                    copy::pick(locale, "Light", "浅色"),
                    !dark,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.session.set_theme(DesktopTheme::Light);
                    cx.notify();
                })),
            );
        let scale = self.session.text_scale();
        let scales = ui::segmented(t)
            .id("settings-text-scale")
            .role(gpui::Role::TabList)
            .aria_label(copy::pick(locale, "Text size", "文字大小"))
            .children([100u8, 110, 125].map(|value| {
                ui::segment(
                    t,
                    ElementId::Name(SharedString::from(format!("settings-scale-{value}"))),
                    format!("{value}%"),
                    scale == value,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.session.set_text_scale(value);
                    cx.notify();
                }))
            }));
        let reduced = self.session.motion() == DesktopMotion::Reduced;
        let motion = ui::segmented(t)
            .id("settings-motion")
            .role(gpui::Role::TabList)
            .aria_label(copy::pick(locale, "Motion", "动效"))
            .child(
                ui::segment(
                    t,
                    "settings-motion-system",
                    copy::pick(locale, "System", "跟随系统"),
                    !reduced,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.session.set_motion(DesktopMotion::System);
                    cx.notify();
                })),
            )
            .child(
                ui::segment(
                    t,
                    "settings-motion-reduced",
                    copy::pick(locale, "Reduced", "减少"),
                    reduced,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.session.set_motion(DesktopMotion::Reduced);
                    cx.notify();
                })),
            );
        let roots = self.session.shell().scan_roots().to_vec();
        let roots_list = if roots.is_empty() {
            ui::any(ui::muted(
                t,
                copy::pick(
                    locale,
                    "No project folders saved yet.",
                    "尚未保存项目文件夹。",
                ),
            ))
        } else {
            ui::any(
                div()
                    .flex()
                    .flex_col()
                    .gap(rems(0.3))
                    .children(roots.into_iter().map(|root| {
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(rems(0.6))
                            .px(rems(0.75))
                            .py(rems(0.5))
                            .rounded(rems(0.5))
                            .bg(rgb(t.surface_alt))
                            .child(ui::icon("folder", t.accent))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(rems(0.86))
                                    .child(root),
                            )
                    })),
            )
        };
        let dirty = self.session.settings_dirty();
        let save = ui::icon_button(
            t,
            "settings-save",
            "check",
            copy::save_label(locale),
            if dirty {
                ButtonKind::Primary
            } else {
                ButtonKind::Secondary
            },
        )
        .on_click(cx.listener(|this, _, window, cx| this.save(window, cx)));
        div()
            .flex()
            .flex_col()
            .gap(rems(1.25))
            .child(self.page_header(
                t,
                "settings",
                copy::settings_title(locale),
                copy::settings_subtitle(locale),
                vec![self.surface_back_button(t, cx), ui::any(save)],
            ))
            .children(self.shared_banners(t))
            .when(dirty, |d| {
                d.child(ui::banner(
                    t,
                    Tone::Info,
                    copy::pick(
                        locale,
                        "You have unsaved changes. Save writes them to your preferences.",
                        "有未保存的更改。保存后写入偏好设置。",
                    ),
                ))
            })
            .child(
                ui::card(t)
                    .py(rems(0.5))
                    .child(self.setting_row(
                        t,
                        copy::language_label(locale),
                        copy::language_option(locale),
                        language,
                    ))
                    .child(ui::divider(t))
                    .child(self.setting_row(
                        t,
                        copy::theme_label(locale),
                        format!(
                            "{} · {}",
                            theme.as_str(),
                            copy::pick(
                                locale,
                                "Windows high contrast always wins",
                                "Windows 高对比度始终优先"
                            )
                        ),
                        appearance,
                    ))
                    .child(ui::divider(t))
                    .child(self.setting_row(
                        t,
                        copy::pick(locale, "Text size", "文字大小"),
                        copy::pick(
                            locale,
                            "Scales text and spacing together.",
                            "文字与间距一同缩放。",
                        ),
                        scales,
                    ))
                    .child(ui::divider(t))
                    .child(self.setting_row(
                        t,
                        copy::pick(locale, "Motion", "动效"),
                        self.session.motion().as_str().to_string(),
                        motion,
                    )),
            )
            .child(
                ui::card(t)
                    .gap(rems(0.75))
                    .child(ui::card_header(t, copy::project_folders(locale)))
                    .child(roots_list)
                    .child(ui::muted(t, copy::separate_protection(locale))),
            )
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn simple_list_page(
        &self,
        t: &Theme,
        icon: &'static str,
        title: &'static str,
        subtitle: &'static str,
        rows: Vec<String>,
        empty: SharedString,
        row_icon: &'static str,
        extra: Option<gpui::AnyElement>,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let list = if rows.is_empty() {
            ui::any(ui::empty_state(t, icon, title, empty))
        } else {
            ui::any(
                div()
                    .id("support-list")
                    .role(gpui::Role::List)
                    .flex()
                    .flex_col()
                    .gap(rems(0.2))
                    .children(rows.into_iter().enumerate().map(|(index, row)| {
                        div()
                            .id(gpui::ElementId::named_usize("support-row", index))
                            .role(gpui::Role::ListItem)
                            .aria_label(SharedString::from(row.clone()))
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(rems(0.65))
                            .px(rems(0.8))
                            .py(rems(0.55))
                            .rounded(rems(0.5))
                            .child(ui::icon(row_icon, t.text_muted))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(rems(0.86))
                                    .child(SharedString::from(row)),
                            )
                    })),
            )
        };
        div()
            .flex()
            .flex_col()
            .gap(rems(1.25))
            .child(self.page_header(
                t,
                icon,
                title,
                subtitle,
                vec![self.surface_back_button(t, cx)],
            ))
            .children(self.shared_banners(t))
            .children(extra)
            .child(ui::card(t).child(list))
            .into_any_element()
    }

    pub(super) fn protection_page(
        &mut self,
        t: &Theme,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let locale = self.session.locale();
        self.simple_list_page(
            t,
            "shield",
            copy::protection_title(locale),
            copy::protection_subtitle(locale),
            self.session.shell().protections().to_vec(),
            copy::pick(
                locale,
                "No protected paths. Protect a row from the Clean review to keep it out of every selection.",
                "暂无受保护路径。在清理审阅中保护某一行，即可让它不进入任何选择。",
            )
            .into(),
            "lock",
            None,
            cx,
        )
    }

    pub(super) fn rules_page(&mut self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        self.simple_list_page(
            t,
            "rules",
            copy::rules_title(locale),
            copy::rules_subtitle(locale),
            self.session.shell().rules().to_vec(),
            copy::rules_empty(locale).into(),
            "code",
            None,
            cx,
        )
    }

    pub(super) fn history_page(&mut self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let (rows, empty) = match self.session.shell().history_store() {
            crate::shell::HistoryStoreView::Unavailable => {
                (Vec::new(), copy::history_unavailable(locale))
            }
            crate::shell::HistoryStoreView::Ready => (
                self.session.shell().history().to_vec(),
                copy::history_empty(locale),
            ),
            crate::shell::HistoryStoreView::NotLoaded | crate::shell::HistoryStoreView::Empty => {
                (Vec::new(), copy::history_empty(locale))
            }
        };
        self.simple_list_page(
            t,
            "history",
            copy::history_title(locale),
            copy::history_subtitle(locale),
            rows,
            empty.into(),
            "history",
            Some(ui::any(ui::banner(
                t,
                Tone::Info,
                copy::history_no_replay(locale),
            ))),
            cx,
        )
    }
}
