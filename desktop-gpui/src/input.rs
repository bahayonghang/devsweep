use std::ops::Range;

use gpui::{
    App, Bounds, Context, Element, ElementId, ElementInputHandler, Entity, EntityInputHandler,
    FocusHandle, Focusable, GlobalElementId, IntoElement, LayoutId, MouseButton, MouseDownEvent,
    PaintQuad, Pixels, ShapedLine, Style, TextAlign, TextRun, UTF16Selection, UnderlineStyle,
    Window, actions, div, fill, hsla, point, prelude::*, px, relative, rgb, size, white,
};

use crate::buffer::{BufferError, TextBuffer};

actions!(spike_input, [Backspace, Delete, Left, Right, Home, End]);

pub struct TextInput {
    focus_handle: FocusHandle,
    buffer: TextBuffer,
    placeholder: &'static str,
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
}

impl TextInput {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, placeholder: &'static str) -> Self {
        let focus_handle = cx.focus_handle().tab_index(1).tab_stop(true);
        window.focus(&focus_handle, cx);
        Self {
            focus_handle,
            buffer: TextBuffer::new(),
            placeholder,
            last_layout: None,
            last_bounds: None,
        }
    }

    pub fn content(&self) -> &str {
        self.buffer.content()
    }

    fn left(&mut self, _: &Left, window: &mut Window, cx: &mut Context<Self>) {
        self.buffer.move_left();
        self.selection_changed(window, cx);
    }

    fn right(&mut self, _: &Right, window: &mut Window, cx: &mut Context<Self>) {
        self.buffer.move_right();
        self.selection_changed(window, cx);
    }

    fn home(&mut self, _: &Home, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.buffer.move_to(0);
        self.selection_changed(window, cx);
    }

    fn end(&mut self, _: &End, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.buffer.move_to(self.buffer.content().len());
        self.selection_changed(window, cx);
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if !self.buffer.backspace() {
            window.play_system_bell();
            return;
        }
        self.text_changed(window, cx);
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if !self.buffer.delete_forward() {
            window.play_system_bell();
            return;
        }
        self.text_changed(window, cx);
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.request_virtual_keyboard();
        let offset = self.index_for_mouse_position(event.position);
        if self.buffer.move_to(offset).is_err() {
            return;
        }
        self.selection_changed(window, cx);
    }

    fn index_for_mouse_position(&self, position: gpui::Point<Pixels>) -> usize {
        let (Some(bounds), Some(line)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };
        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.buffer.content().len();
        }
        line.closest_index_for_x(position.x - bounds.left())
    }

    fn selection_changed(&mut self, window: &Window, cx: &mut Context<Self>) {
        window.notify_text_selection_changed(&self.focus_handle);
        cx.notify();
    }

    fn text_changed(&mut self, window: &Window, cx: &mut Context<Self>) {
        window.notify_text_input_changed(&self.focus_handle);
        cx.notify();
    }

    fn apply(
        &mut self,
        result: Result<(), BufferError>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if result.is_ok() {
            self.text_changed(window, cx);
        }
    }
}

impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let text = self.buffer.text_for_utf16_range(range_utf16.clone()).ok()?;
        *actual_range = Some(range_utf16);
        Some(text)
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.buffer.selected_utf16(),
            reversed: false,
        })
    }

    fn set_selected_text_range(
        &mut self,
        range_utf16: Range<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.buffer.set_selected_utf16(range_utf16).is_ok() {
            self.selection_changed(window, cx);
        }
    }

    fn text_length_utf16(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        Some(self.buffer.utf16_len())
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.buffer.marked_utf16()
    }

    fn unmark_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.buffer.unmark();
        self.text_changed(window, cx);
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let result = self.buffer.replace_utf16(range_utf16, new_text);
        self.apply(result, window, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let result =
            self.buffer
                .replace_and_mark_utf16(range_utf16, new_text, new_selected_range_utf16);
        self.apply(result, window, cx);
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let last_layout = self.last_layout.as_ref()?;
        if last_layout.text != self.buffer.content() {
            return None;
        }
        let utf8 = self.buffer.byte_range_from_utf16(range_utf16).ok()?;
        Some(Bounds::from_corners(
            point(
                bounds.left() + last_layout.x_for_index(utf8.start),
                bounds.top(),
            ),
            point(
                bounds.left() + last_layout.x_for_index(utf8.end),
                bounds.bottom(),
            ),
        ))
    }

    fn selection_bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<Bounds<Pixels>> {
        self.bounds_for_range(range_utf16, bounds, window, cx)
            .into_iter()
            .collect()
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let line_point = self.last_bounds?.localize(&point)?;
        let last_layout = self.last_layout.as_ref()?;
        if last_layout.text != self.buffer.content() {
            return None;
        }
        let content = self.buffer.content();
        let mut utf8_index = last_layout.closest_index_for_x(line_point.x);
        if utf8_index > content.len() {
            utf8_index = content.len();
        }
        while utf8_index > 0 && !content.is_char_boundary(utf8_index) {
            utf8_index -= 1;
        }
        Some(content[..utf8_index].encode_utf16().count())
    }
}

struct TextElement {
    input: Entity<TextInput>,
}

struct PrepaintState {
    line: Option<ShapedLine>,
    placeholder: Option<ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let content = input.content().to_string();
        let placeholder = input.placeholder.to_string();
        let selected_range = input.buffer.selected_range();
        let cursor = input.buffer.cursor_offset();
        let marked_range = input.buffer.marked_range();
        let style = window.text_style();
        let run = TextRun {
            len: content.len(),
            font: style.font(),
            color: style.color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = if let Some(marked_range) = marked_range.as_ref() {
            vec![
                TextRun {
                    len: marked_range.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked_range.end.saturating_sub(marked_range.start),
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.0),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: content.len().saturating_sub(marked_range.end),
                    ..run
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![run]
        };
        let font_size = style.font_size.to_pixels(window.rem_size());
        let line = window
            .text_system()
            .shape_line(content.clone().into(), font_size, &runs, None);
        let placeholder_line = if content.is_empty() {
            let placeholder_run = TextRun {
                len: placeholder.len(),
                font: style.font(),
                color: hsla(0., 0., 0., 0.45),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            Some(window.text_system().shape_line(
                placeholder.into(),
                font_size,
                &[placeholder_run],
                None,
            ))
        } else {
            None
        };
        let cursor_pos = line.x_for_index(cursor);
        let (selection, cursor) = if selected_range.is_empty() {
            (
                None,
                Some(fill(
                    Bounds::new(
                        point(bounds.left() + cursor_pos, bounds.top()),
                        size(px(2.), bounds.bottom() - bounds.top()),
                    ),
                    gpui::blue(),
                )),
            )
        } else {
            (
                Some(fill(
                    Bounds::from_corners(
                        point(
                            bounds.left() + line.x_for_index(selected_range.start),
                            bounds.top(),
                        ),
                        point(
                            bounds.left() + line.x_for_index(selected_range.end),
                            bounds.bottom(),
                        ),
                    ),
                    rgb(0x3311ff30),
                )),
                None,
            )
        };
        PrepaintState {
            line: Some(line),
            placeholder: placeholder_line,
            cursor,
            selection,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection);
        }
        let line = prepaint.line.take().unwrap();
        if let Some(placeholder) = prepaint.placeholder.take() {
            placeholder
                .paint(
                    bounds.origin,
                    window.line_height(),
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                )
                .ok();
        } else {
            line.paint(
                bounds.origin,
                window.line_height(),
                TextAlign::Left,
                None,
                window,
                cx,
            )
            .ok();
        }
        if focus_handle.is_focused(window)
            && let Some(cursor) = prepaint.cursor.take()
        {
            window.paint_quad(cursor);
        }
        self.input.update(cx, |input, _cx| {
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
        });
    }
}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TextInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("note-input")
            .role(gpui::Role::TextInput)
            .aria_label("Note")
            .aria_value(self.content())
            .aria_placeholder(self.placeholder)
            .key_context("SpikeInput")
            .track_focus(&self.focus_handle)
            .tab_index(1)
            .tab_stop(true)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .w_full()
            .px_3()
            .py_2()
            .bg(white())
            .border_1()
            .border_color(rgb(0xc8c2b4))
            .rounded_md()
            .text_size(px(20.))
            .line_height(px(28.))
            .focus(|style| style.border_2().border_color(rgb(0x1d4e89)))
            .child(TextElement { input: cx.entity() })
    }
}
