use std::ops::Range;
use std::time::Duration;

use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, Element, ElementId, ElementInputHandler,
    ExternalPaths, GlobalElementId, Hsla, LayoutId, Pixels, Point, ShapedLine, SharedString,
    Style, TextRun, UnderlineStyle, Window, fill, point, prelude::*, px, relative, size,
};

use super::SidePanelLeft;

pub const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(530);
const CURSOR_WIDTH: f32 = 1.5;

#[derive(Clone, Debug)]
pub struct TextInputState {
    pub content: SharedString,
    pub selected_range: Range<usize>,
    pub selection_reversed: bool,
    pub cursor_visible: bool,
    pub(crate) marked_range: Option<Range<usize>>,
    is_selecting: bool,
    pub has_drop_hover: bool,
}

impl TextInputState {
    pub fn new() -> Self {
        Self {
            content: SharedString::default(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            cursor_visible: true,
            is_selecting: false,
            has_drop_hover: false,
        }
    }

    pub fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    pub fn clear(&mut self) {
        self.content = SharedString::default();
        self.selected_range = 0..0;
        self.selection_reversed = false;
        self.marked_range = None;
        self.cursor_visible = true;
        self.is_selecting = false;
    }

    fn effective_range(&self) -> Range<usize> {
        self.marked_range.clone().unwrap_or(self.selected_range.clone())
    }

    pub fn move_to(&mut self, offset: usize) {
        self.selected_range = offset..offset;
        self.selection_reversed = false;
    }

    pub fn select_to(&mut self, offset: usize) {
        if self.selection_reversed {
            self.selected_range.start = offset;
        } else {
            self.selected_range.end = offset;
        }
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
    }

    pub(crate) fn replace_range(&mut self, range: Range<usize>, new_text: &str) {
        self.content = (self.content[..range.start].to_owned() + new_text + &self.content[range.end..]).into();
        let new_cursor = range.start + new_text.len();
        self.selected_range = new_cursor..new_cursor;
        self.marked_range = None;
    }

    pub fn insert_char(&mut self, ch: &str) {
        self.replace_range(self.effective_range(), ch);
    }

    pub fn backspace(&mut self) {
        if !self.selected_range.is_empty() {
            self.insert_char("");
            return;
        }
        let cursor = self.cursor_offset();
        let prev = prev_char_boundary(&self.content, cursor);
        if cursor == prev {
            return;
        }
        self.replace_range(prev..cursor, "");
    }

    pub fn delete_forward(&mut self) {
        if !self.selected_range.is_empty() {
            self.insert_char("");
            return;
        }
        let cursor = self.cursor_offset();
        let next = next_char_boundary(&self.content, cursor);
        if cursor == next {
            return;
        }
        self.replace_range(cursor..next, "");
    }

    pub fn cursor_left(&mut self) {
        if !self.selected_range.is_empty() {
            self.move_to(self.selected_range.start);
        } else {
            let cursor = self.cursor_offset();
            self.move_to(prev_char_boundary(&self.content, cursor));
        }
    }

    pub fn cursor_right(&mut self) {
        if !self.selected_range.is_empty() {
            self.move_to(self.selected_range.end);
        } else {
            let cursor = self.cursor_offset();
            self.move_to(next_char_boundary(&self.content, cursor));
        }
    }

    pub fn select_left(&mut self) {
        let cursor = self.cursor_offset();
        self.select_to(prev_char_boundary(&self.content, cursor));
    }

    pub fn select_right(&mut self) {
        let cursor = self.cursor_offset();
        self.select_to(next_char_boundary(&self.content, cursor));
    }

    pub fn home(&mut self) { self.move_to(0); }
    pub fn end(&mut self) { self.move_to(self.content.len()); }
    pub fn select_home(&mut self) { self.select_to(0); }
    pub fn select_end(&mut self) { self.select_to(self.content.len()); }

    pub fn select_all(&mut self) {
        let len = self.content.len();
        self.selected_range = 0..len;
        self.selection_reversed = false;
    }

    pub fn cursor_left_word(&mut self) {
        let cursor = self.cursor_offset();
        self.move_to(prev_word_boundary(&self.content, cursor));
    }

    pub fn cursor_right_word(&mut self) {
        let cursor = self.cursor_offset();
        self.move_to(next_word_boundary(&self.content, cursor));
    }

    pub fn select_left_word(&mut self) {
        let cursor = self.cursor_offset();
        self.select_to(prev_word_boundary(&self.content, cursor));
    }

    pub fn select_right_word(&mut self) {
        let cursor = self.cursor_offset();
        self.select_to(next_word_boundary(&self.content, cursor));
    }

    pub fn delete_word_backward(&mut self) {
        let cursor = self.cursor_offset();
        let prev = prev_word_boundary(&self.content, cursor);
        if cursor == prev { return; }
        if self.selected_range.is_empty() {
            self.select_to(prev);
        }
        self.insert_char("");
    }

    pub fn copy_selection(&self, cx: &mut Context<SidePanelLeft>) {
        if !self.selected_range.is_empty() {
            let text = self.content[self.selected_range.clone()].to_string();
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    pub fn cut_selection(&mut self, cx: &mut Context<SidePanelLeft>) {
        if !self.selected_range.is_empty() {
            let text = self.content[self.selected_range.clone()].to_string();
            cx.write_to_clipboard(ClipboardItem::new_string(text));
            self.replace_range(self.selected_range.clone(), "");
        }
    }

    pub fn paste(&mut self, cx: &mut Context<SidePanelLeft>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_range(self.effective_range(), &text);
        }
    }

    pub fn index_for_mouse_position(
        &self,
        position: Point<Pixels>,
        bounds: &Bounds<Pixels>,
        line: &ShapedLine,
    ) -> usize {
        if self.content.is_empty() { return 0; }
        if position.y < bounds.top() { return 0; }
        if position.y > bounds.bottom() { return self.content.len(); }
        line.closest_index_for_x(position.x - bounds.left())
    }

    pub fn on_mouse_down(&mut self, offset: usize, shift: bool) {
        self.is_selecting = true;
        if shift { self.select_to(offset); }
        else { self.move_to(offset); }
    }

    pub fn on_mouse_up(&mut self) { self.is_selecting = false; }

    pub fn on_mouse_move(&mut self, offset: usize) {
        if self.is_selecting { self.select_to(offset); }
    }

    pub(crate) fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8 = 0;
        let mut utf16 = 0;
        for ch in self.content.chars() {
            if utf16 >= offset { break; }
            utf16 += ch.len_utf16();
            utf8 += ch.len_utf8();
        }
        utf8
    }

    pub(crate) fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16 = 0;
        let mut utf8 = 0;
        for ch in self.content.chars() {
            if utf8 >= offset { break; }
            utf8 += ch.len_utf8();
            utf16 += ch.len_utf16();
        }
        utf16
    }

    pub(crate) fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    pub(crate) fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }
}

fn prev_char_boundary(content: &str, offset: usize) -> usize {
    let offset = offset.min(content.len());
    if offset == 0 {
        return 0;
    }
    content[..offset]
        .char_indices()
        .rev()
        .map(|(i, _)| i)
        .next()
        .unwrap_or(0)
}

fn next_char_boundary(content: &str, offset: usize) -> usize {
    content[offset..]
        .char_indices()
        .skip(1)
        .map(|(i, _)| offset + i)
        .next()
        .unwrap_or(content.len())
}

pub fn prev_word_boundary(content: &str, offset: usize) -> usize {
    if offset == 0 { return 0; }
    let mut chars = content[..offset].char_indices().rev().peekable();
    // Skip any non-word chars immediately before cursor
    while let Some(&(_, ch)) = chars.peek() {
        if is_word_char(ch) { break; }
        chars.next();
    }
    // Skip word chars to find the word start
    while let Some(&(i, ch)) = chars.peek() {
        if !is_word_char(ch) { return i + ch.len_utf8(); }
        chars.next();
    }
    0
}

pub fn next_word_boundary(content: &str, offset: usize) -> usize {
    let len = content.len();
    if offset >= len { return len; }
    let mut chars = content[offset..].char_indices().peekable();
    // Skip any non-word chars immediately after cursor
    while let Some(&(_, ch)) = chars.peek() {
        if is_word_char(ch) { break; }
        chars.next();
    }
    // Skip word chars to find the word end
    while let Some(&(i, ch)) = chars.peek() {
        if !is_word_char(ch) { return offset + i; }
        chars.next();
    }
    len
}

/// Unicode-aware word character — alphanumeric in any script (not just ASCII),
/// plus underscores, hyphens, and non-breaking glue.
fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric()
        || matches!(ch, '_' | '-' | '\u{202F}' | '\u{00A0}' | '\u{2011}')
}

pub struct TextInputElement {
    pub content: SharedString,
    pub selected_range: Range<usize>,
    pub selection_reversed: bool,
    pub cursor_visible: bool,
    pub is_focused: bool,
    pub entity: gpui::WeakEntity<SidePanelLeft>,
}

pub struct PrepaintState {
    line: Option<ShapedLine>,
    cursor: Option<gpui::PaintQuad>,
    selection: Option<gpui::PaintQuad>,
}

impl IntoElement for TextInputElement {
    type Element = Self;
    fn into_element(self) -> Self::Element { self }
}

impl Element for TextInputElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> { None }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> { None }

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
        let display_text = self.content.clone();
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());

        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: style.color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let runs = if let Some(mr) = self.marked_range_from_input(cx) {
            vec![
                TextRun { len: mr.start, ..run.clone() },
                TextRun {
                    len: mr.end - mr.start,
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.0),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun { len: display_text.len() - mr.end, ..run },
            ].into_iter().filter(|r| r.len > 0).collect()
        } else {
            vec![run]
        };

        let line = window
            .text_system()
            .shape_line(display_text, font_size, &runs, None);

        let cursor_offset = if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        };
        let cursor_pos = line.x_for_index(cursor_offset);
        let sel_color = Hsla { h: 0.6, s: 0.5, l: 0.5, a: 0.25 };

        let (selection, cursor) = if self.selected_range.is_empty() {
            (None, Some(
                fill(
                    Bounds::new(
                        point(bounds.left() + cursor_pos, bounds.top()),
                        size(px(CURSOR_WIDTH), bounds.bottom() - bounds.top()),
                    ),
                    if self.cursor_visible { style.color } else { Hsla::default() },
                )
            ))
        } else {
            let sx = line.x_for_index(self.selected_range.start);
            let ex = line.x_for_index(self.selected_range.end);
            (Some(fill(
                Bounds::from_corners(
                    point(bounds.left() + sx.min(ex), bounds.top()),
                    point(bounds.left() + sx.max(ex), bounds.bottom()),
                ),
                sel_color,
            )), None)
        };

        PrepaintState { line: Some(line), cursor, selection }
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
        let line = prepaint.line.take();
        let Some(entity) = self.entity.upgrade() else { return };
        let focus_handle = entity.read(cx).composer_focus.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, entity.clone()),
            cx,
        );
        if let Some(sel) = prepaint.selection.take() {
            window.paint_quad(sel);
        }
        if let Some(ref l) = line {
            let _ = l.paint(
                bounds.origin,
                window.line_height(),
                gpui::TextAlign::Left,
                None,
                window,
                cx,
            );
        }
        if self.is_focused && let Some(cursor) = prepaint.cursor.take() {
            window.paint_quad(cursor);
        }
        entity.update(cx, |this, _cx| {
            this.composer_last_layout = line;
            this.composer_last_bounds = Some(bounds);
        });
    }
}

impl TextInputElement {
    fn marked_range_from_input(&self, cx: &App) -> Option<Range<usize>> {
        self.entity
            .upgrade()
            .and_then(|e| e.read(cx).composer_input.marked_range.clone())
    }
}