//! Test-only native projection of the production host-action bar.
//! The ShellView itself is never mounted, so it cannot start its render workers.
use super::*;

struct HostReceipt {
    shell: Entity<ShellView>,
    theme: Theme,
}

impl Render for HostReceipt {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_rem_size(px(16.));
        let bar = self
            .shell
            .update(cx, |shell, cx| shell.host_review_bar(&self.theme, cx));
        div()
            .size_full()
            .p(ui::SPACE_4)
            .text_color(rgb(self.theme.ink))
            .bg(self.theme.sheet_solid.color())
            .font_family(ui::FONT_FAMILY)
            .text_size(ui::TYPE_BODY)
            .children(bar)
    }
}

pub(crate) fn run_host_receipt(session: Session) {
    application().with_assets(PackagedAssets).run(move |cx| {
        cx.text_system()
            .add_fonts(vec![Cow::Borrowed(ui::FONT_BYTES)])
            .expect("fixture font");
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        let theme = Theme::for_preferences(session.theme(), false, false);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(860.), px(340.)),
                    cx,
                ))),
                titlebar: Some(TitlebarOptions {
                    title: Some(WINDOW_TITLE.into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            move |_, cx| {
                let input = cx.new(|cx| {
                    TextInput::new_secondary(
                        cx,
                        "",
                        "receipt-unused",
                        accessible_control(session.locale(), AccessibleSlot::CleanSearch),
                    )
                });
                let shell = cx.new(|cx| ShellView {
                    session,
                    focus_handle: cx.focus_handle(),
                    material: WindowMaterial::default(),
                    bundled_font: true,
                    save_error: None,
                    search: input.clone(),
                    font_search: input,
                    fonts: crate::fonts::FontCatalogue::default(),
                    software_sort_open: false,
                    analyze_menu: None,
                    clean_page_scroll: gpui::ScrollHandle::new(),
                    clean_rows_scroll: gpui::ScrollHandle::new(),
                });
                cx.new(|_| HostReceipt { shell, theme })
            },
        )
        .expect("open receipt window");
    });
}
