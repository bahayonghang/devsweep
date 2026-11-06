use std::ops::Range;

/// UTF-8 text with a UTF-16 selection and an optional IME marked range.
///
/// Offsets that are not on a Unicode scalar boundary are rejected. Backspace
/// and arrow movement step by scalar values, not grapheme clusters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextBuffer {
    content: String,
    selected: Range<usize>,
    selection_reversed: bool,
    marked: Option<Range<usize>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferError {
    InvertedRange,
    OutOfBounds,
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            selected: 0..0,
            selection_reversed: false,
            marked: None,
        }
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn selected_range(&self) -> Range<usize> {
        self.selected.clone()
    }

    pub fn marked_range(&self) -> Option<Range<usize>> {
        self.marked.clone()
    }

    pub fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected.start
        } else {
            self.selected.end
        }
    }

    pub fn utf16_len(&self) -> usize {
        self.content.encode_utf16().count()
    }

    pub fn text_for_utf16_range(&self, range: Range<usize>) -> Result<String, BufferError> {
        let utf8 = self.byte_range_from_utf16(range)?;
        Ok(self.slice(utf8)?.to_string())
    }

    pub fn byte_range_from_utf16(&self, range: Range<usize>) -> Result<Range<usize>, BufferError> {
        self.range_from_utf16(&range)
    }

    pub fn selected_utf16(&self) -> Range<usize> {
        self.range_to_utf16(&self.selected)
    }

    pub fn marked_utf16(&self) -> Option<Range<usize>> {
        self.marked.as_ref().map(|range| self.range_to_utf16(range))
    }

    pub fn set_selected_utf16(&mut self, range: Range<usize>) -> Result<(), BufferError> {
        self.selected = self.range_from_utf16(&range)?;
        self.selection_reversed = false;
        Ok(())
    }

    pub fn unmark(&mut self) {
        self.marked = None;
    }

    pub fn select_all(&mut self) {
        self.selected = 0..self.content.len();
        self.selection_reversed = false;
        self.marked = None;
    }

    pub fn replace_all(&mut self, text: &str) {
        self.replace_utf16(Some(0..self.utf16_len()), text)
            .expect("whole buffer is a valid UTF-16 range");
    }

    pub fn replace_utf16(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
    ) -> Result<(), BufferError> {
        let range = self.resolve_range(range_utf16)?;
        self.splice(range, new_text)?;
        self.marked = None;
        Ok(())
    }

    pub fn replace_and_mark_utf16(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_utf16: Option<Range<usize>>,
    ) -> Result<(), BufferError> {
        let range = self.resolve_range(range_utf16)?;
        let prefix = self
            .content
            .get(..range.start)
            .ok_or(BufferError::OutOfBounds)?;
        let suffix = self
            .content
            .get(range.end..)
            .ok_or(BufferError::OutOfBounds)?;
        let mut content = String::with_capacity(prefix.len() + new_text.len() + suffix.len());
        content.push_str(prefix);
        content.push_str(new_text);
        content.push_str(suffix);
        let insertion_start = prefix.len();
        let insertion_end = insertion_start + new_text.len();
        let marked = if new_text.is_empty() {
            None
        } else {
            Some(insertion_start..insertion_end)
        };
        let selected = if let Some(selected) = new_selected_utf16.as_ref() {
            if selected.end < selected.start {
                return Err(BufferError::InvertedRange);
            }
            let start_utf16 = prefix.encode_utf16().count();
            let inserted = new_text.encode_utf16().count();
            let clamped = start_utf16 + selected.start.min(inserted)
                ..start_utf16 + selected.end.min(inserted);
            byte_range_in(&content, &clamped)?
        } else {
            insertion_end..insertion_end
        };
        self.content = content;
        self.marked = marked;
        self.selected = selected;
        self.selection_reversed = false;
        Ok(())
    }

    pub fn move_to(&mut self, offset: usize) -> Result<(), BufferError> {
        if self.content.get(offset..).is_none() && offset != self.content.len() {
            return Err(BufferError::OutOfBounds);
        }
        if offset < self.content.len() && !self.content.is_char_boundary(offset) {
            return Err(BufferError::OutOfBounds);
        }
        self.selected = offset..offset;
        self.selection_reversed = false;
        Ok(())
    }

    pub fn move_left(&mut self) {
        if self.selected.is_empty() {
            let cursor = self.cursor_offset();
            let previous = self.content[..cursor]
                .chars()
                .next_back()
                .map(|ch| cursor - ch.len_utf8())
                .unwrap_or(0);
            self.selected = previous..previous;
        } else {
            let start = self.selected.start;
            self.selected = start..start;
        }
        self.selection_reversed = false;
    }

    pub fn move_right(&mut self) {
        if self.selected.is_empty() {
            let cursor = self.cursor_offset();
            let next = self.content[cursor..]
                .chars()
                .next()
                .map(|ch| cursor + ch.len_utf8())
                .unwrap_or(cursor);
            self.selected = next..next;
        } else {
            let end = self.selected.end;
            self.selected = end..end;
        }
        self.selection_reversed = false;
    }

    pub fn backspace(&mut self) -> bool {
        if self.selected.is_empty() {
            let cursor = self.cursor_offset();
            let Some(previous) = self.content[..cursor].chars().next_back() else {
                return false;
            };
            self.selected = cursor - previous.len_utf8()..cursor;
        }
        self.splice(self.selected.clone(), "").is_ok()
    }

    pub fn delete_forward(&mut self) -> bool {
        if self.selected.is_empty() {
            let cursor = self.cursor_offset();
            let Some(next) = self.content[cursor..].chars().next() else {
                return false;
            };
            self.selected = cursor..cursor + next.len_utf8();
        }
        self.splice(self.selected.clone(), "").is_ok()
    }

    fn resolve_range(
        &self,
        range_utf16: Option<Range<usize>>,
    ) -> Result<Range<usize>, BufferError> {
        if let Some(range) = range_utf16 {
            self.range_from_utf16(&range)
        } else if let Some(marked) = self.marked.clone() {
            Ok(marked)
        } else {
            Ok(self.selected.clone())
        }
    }

    fn splice(&mut self, range: Range<usize>, new_text: &str) -> Result<(), BufferError> {
        let prefix = self
            .content
            .get(..range.start)
            .ok_or(BufferError::OutOfBounds)?;
        let suffix = self
            .content
            .get(range.end..)
            .ok_or(BufferError::OutOfBounds)?;
        let mut content = String::with_capacity(prefix.len() + new_text.len() + suffix.len());
        content.push_str(prefix);
        content.push_str(new_text);
        content.push_str(suffix);
        self.content = content;
        let caret = range.start + new_text.len();
        self.selected = caret..caret;
        self.selection_reversed = false;
        Ok(())
    }

    fn slice(&self, range: Range<usize>) -> Result<&str, BufferError> {
        self.content.get(range).ok_or(BufferError::OutOfBounds)
    }

    fn range_from_utf16(&self, range: &Range<usize>) -> Result<Range<usize>, BufferError> {
        byte_range_in(&self.content, range)
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        offset_to_utf16(&self.content, offset)
    }
}

fn byte_range_in(content: &str, range: &Range<usize>) -> Result<Range<usize>, BufferError> {
    if range.end < range.start {
        return Err(BufferError::InvertedRange);
    }
    Ok(offset_from_utf16(content, range.start)?..offset_from_utf16(content, range.end)?)
}

fn offset_from_utf16(content: &str, offset: usize) -> Result<usize, BufferError> {
    let mut utf8_offset = 0;
    let mut utf16_count = 0;
    for ch in content.chars() {
        if utf16_count == offset {
            return Ok(utf8_offset);
        }
        if utf16_count > offset {
            return Err(BufferError::OutOfBounds);
        }
        utf16_count += ch.len_utf16();
        utf8_offset += ch.len_utf8();
    }
    if utf16_count == offset {
        Ok(utf8_offset)
    } else {
        Err(BufferError::OutOfBounds)
    }
}

fn offset_to_utf16(content: &str, offset: usize) -> usize {
    let mut utf16_offset = 0;
    let mut utf8_count = 0;
    for ch in content.chars() {
        if utf8_count >= offset {
            break;
        }
        utf8_count += ch.len_utf8();
        utf16_offset += ch.len_utf16();
    }
    utf16_offset
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_all_then_paste_replaces_unicode_and_clears_composition() {
        let mut buffer = TextBuffer::new();
        buffer.replace_utf16(None, "Segoe 字体🦫").unwrap();
        buffer.replace_and_mark_utf16(None, "ni", None).unwrap();
        buffer.select_all();
        assert_eq!(buffer.selected_utf16(), 0..buffer.utf16_len());
        assert_eq!(buffer.marked_range(), None);
        buffer.replace_utf16(None, "Consolas").unwrap();
        assert_eq!(buffer.content(), "Consolas");
        assert_eq!(buffer.selected_utf16(), 8..8);
        buffer.replace_utf16(None, " 字体").unwrap();
        assert_eq!(buffer.content(), "Consolas 字体");
    }

    #[test]
    fn accessible_value_replaces_entire_buffer_independent_of_selection_or_mark() {
        let mut buffer = TextBuffer::new();
        buffer.replace_utf16(None, "A🦫B").unwrap();
        buffer
            .replace_and_mark_utf16(Some(1..3), "输入", Some(0..1))
            .unwrap();
        buffer.replace_all("字体🦫");
        assert_eq!(buffer.content(), "字体🦫");
        assert_eq!(buffer.selected_utf16(), 4..4);
        assert_eq!(buffer.marked_range(), None);
        buffer.replace_all("");
        assert_eq!(buffer.content(), "");
        assert_eq!(buffer.selected_utf16(), 0..0);
    }

    #[test]
    fn insert_and_partial_replace_keep_the_surrounding_text() {
        let mut buffer = TextBuffer::new();
        buffer.replace_utf16(None, "ab你d").unwrap();
        buffer.replace_utf16(Some(2..3), "好").unwrap();
        assert_eq!(buffer.content(), "ab好d");
        assert_eq!(buffer.marked_range(), None);
        assert_eq!(buffer.cursor_offset(), "ab好".len());
    }

    #[test]
    fn composition_mark_then_commit_replaces_only_the_marked_span() {
        let mut buffer = TextBuffer::new();
        buffer
            .replace_and_mark_utf16(None, "ni", Some(0..2))
            .unwrap();
        assert_eq!(buffer.content(), "ni");
        assert_eq!(buffer.marked_range(), Some(0..2));

        buffer
            .replace_and_mark_utf16(Some(0..2), "你", None)
            .unwrap();
        assert_eq!(buffer.content(), "你");
        assert_eq!(buffer.marked_range(), Some(0.."你".len()));

        buffer.replace_utf16(None, "你").unwrap();
        assert_eq!(buffer.content(), "你");
        assert_eq!(buffer.marked_range(), None);
        assert_eq!(buffer.selected_utf16(), 1..1);
    }

    #[test]
    fn empty_composition_clears_the_mark_and_keeps_committed_text() {
        let mut buffer = TextBuffer::new();
        buffer.replace_utf16(None, "a").unwrap();
        buffer.replace_and_mark_utf16(None, "ni", None).unwrap();
        assert_eq!(buffer.content(), "ani");
        buffer.replace_and_mark_utf16(None, "", None).unwrap();
        assert_eq!(buffer.content(), "a");
        assert_eq!(buffer.marked_range(), None);
    }

    #[test]
    fn rejected_ranges_leave_the_buffer_unchanged() {
        let mut buffer = TextBuffer::new();
        buffer
            .replace_and_mark_utf16(None, "ni", Some(0..2))
            .unwrap();
        let before = buffer.clone();

        // The range endpoints are bound to variables so the deliberately
        // inverted ranges below are not statically visible to the compiler.
        let inverted_start = 2;
        let inverted_end = 1;
        assert_eq!(
            buffer.replace_utf16(Some(inverted_start..inverted_end), "x"),
            Err(BufferError::InvertedRange)
        );
        assert_eq!(
            buffer.replace_utf16(Some(5..6), "x"),
            Err(BufferError::OutOfBounds)
        );
        let inverted_text_start = 1;
        let inverted_text_end = 0;
        assert_eq!(
            buffer
                .text_for_utf16_range(inverted_text_start..inverted_text_end)
                .unwrap_err(),
            BufferError::InvertedRange
        );
        assert_eq!(buffer, before);

        let mut emoji = TextBuffer::new();
        emoji.replace_utf16(None, "😀").unwrap();
        let unchanged = emoji.clone();
        assert_eq!(
            emoji.text_for_utf16_range(1..2).unwrap_err(),
            BufferError::OutOfBounds
        );
        assert_eq!(emoji, unchanged);
    }

    #[test]
    fn backspace_at_the_start_fails_without_deleting() {
        let mut buffer = TextBuffer::new();
        buffer.replace_utf16(None, "你").unwrap();
        buffer.move_to(0).unwrap();
        assert!(!buffer.backspace());
        assert_eq!(buffer.content(), "你");
        buffer.move_to(buffer.content().len()).unwrap();
        assert!(buffer.backspace());
        assert_eq!(buffer.content(), "");
        assert!(!buffer.delete_forward());
    }

    #[test]
    fn utf16_roundtrip_reads_one_han_character() {
        let mut buffer = TextBuffer::new();
        buffer.replace_utf16(None, "a你b").unwrap();
        assert_eq!(buffer.text_for_utf16_range(1..2).unwrap(), "你");
        assert_eq!(
            buffer.byte_range_from_utf16(1..2).unwrap(),
            1..1 + "你".len()
        );
        assert_eq!(buffer.utf16_len(), 3);
    }
}
