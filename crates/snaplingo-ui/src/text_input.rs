//! A single-line field's committed selection and separate IME draft. Geometry,
//! clipboard transfer and persistence belong to the consuming UI and host.
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Movement {
    Left,
    Right,
    WordLeft,
    WordRight,
    Home,
    End,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Composition {
    text: String,
    selection: Option<Range<usize>>,
    replacement: Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEdit {
    text: String,
    anchor: usize,
    caret: usize,
    composition: Option<Composition>,
    multiline: bool,
}

impl TextEdit {
    pub fn new(text: String) -> Self {
        let end = text.len();
        Self {
            text,
            anchor: end,
            caret: end,
            composition: None,
            multiline: false,
        }
    }
    pub fn multiline(text: String) -> Self {
        Self {
            multiline: true,
            ..Self::new(text)
        }
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn selection(&self) -> Range<usize> {
        self.anchor.min(self.caret)..self.anchor.max(self.caret)
    }
    pub fn caret(&self) -> usize {
        self.caret
    }
    pub fn move_to(&mut self, byte: usize, extend: bool) -> bool {
        if self.is_composing() || !boundary(&self.text, byte) {
            return false;
        }
        let previous = (self.anchor, self.caret);
        self.composition = None;
        self.caret = byte;
        if !extend {
            self.anchor = byte;
        }
        previous != (self.anchor, self.caret)
    }
    /// Selection offsets are UTF-8 bytes at whole-grapheme boundaries. Invalid
    /// renderer/host offsets leave the current selection intact.
    pub fn set_selection(&mut self, selection: Range<usize>) -> bool {
        if selection.start > selection.end
            || !boundary(&self.text, selection.start)
            || !boundary(&self.text, selection.end)
        {
            return false;
        }
        self.anchor = selection.start;
        self.caret = selection.end;
        self.composition = None;
        true
    }
    pub fn selected_text(&self) -> &str {
        &self.text[self.selection()]
    }
    pub fn select_all(&mut self) {
        self.anchor = 0;
        self.caret = self.text.len();
        self.composition = None;
    }
    pub fn insert(&mut self, text: &str) -> bool {
        self.composition = None;
        self.replace(self.selection(), text)
    }
    /// A nonempty draft captures the committed replacement range once. Empty
    /// preedit clears its visible draft; the replacement remains available for
    /// the Commit event that winit may deliver immediately afterward.
    pub fn preedit(&mut self, text: &str, selection: Option<Range<usize>>) -> bool {
        if let Some(selection) = &selection {
            if selection.start > selection.end
                || !text.is_char_boundary(selection.start)
                || !text.is_char_boundary(selection.end)
            {
                return false;
            }
        }
        if text.is_empty() && self.composition.is_none() {
            return false;
        }
        let replacement = self
            .composition
            .as_ref()
            .map(|draft| draft.replacement.clone())
            .unwrap_or_else(|| self.selection());
        let draft = Composition {
            text: text.into(),
            selection,
            replacement,
        };
        let changed = self.composition.as_ref() != Some(&draft);
        self.composition = Some(draft);
        changed
    }
    pub fn is_composing(&self) -> bool {
        self.composition
            .as_ref()
            .is_some_and(|draft| !draft.text.is_empty())
    }
    /// Cancel on focus loss, Escape handled by the IME, or Ime::Disabled. The
    /// committed selection and string have never been changed by preedit.
    pub fn clear_preedit(&mut self) -> bool {
        self.composition.take().is_some()
    }
    pub fn commit(&mut self, text: &str) -> bool {
        let replacement = self
            .composition
            .take()
            .map(|draft| draft.replacement)
            .unwrap_or_else(|| self.selection());
        self.replace(replacement, text)
    }
    pub fn display_text(&self) -> String {
        if let Some(draft) = self
            .composition
            .as_ref()
            .filter(|draft| !draft.text.is_empty())
        {
            let mut text = self.text.clone();
            text.replace_range(draft.replacement.clone(), &draft.text);
            text
        } else {
            self.text.clone()
        }
    }
    pub fn composition_range(&self) -> Option<Range<usize>> {
        self.composition
            .as_ref()
            .filter(|draft| !draft.text.is_empty())
            .map(|draft| draft.replacement.start..draft.replacement.start + draft.text.len())
    }
    /// Layout shapes display_text and uses these exact byte offsets for caret
    /// and selection. None is the IME's explicit request to hide its cursor.
    pub fn display_selection(&self) -> Option<Range<usize>> {
        if let Some(draft) = self
            .composition
            .as_ref()
            .filter(|draft| !draft.text.is_empty())
        {
            draft.selection.as_ref().map(|selection| {
                draft.replacement.start + selection.start..draft.replacement.start + selection.end
            })
        } else {
            Some(self.selection())
        }
    }
    pub fn display_caret(&self) -> Option<usize> {
        if let Some(draft) = self
            .composition
            .as_ref()
            .filter(|draft| !draft.text.is_empty())
        {
            draft
                .selection
                .as_ref()
                .map(|selection| draft.replacement.start + selection.end)
        } else {
            Some(self.caret)
        }
    }
    /// Directional movement and deletion are held while composing; the system
    /// IME owns those keys until Commit or cancellation.
    pub fn move_caret(&mut self, movement: Movement, extend: bool) -> bool {
        if self.is_composing() {
            return false;
        }
        self.composition = None;
        let previous = (self.anchor, self.caret);
        let selection = self.selection();
        self.caret = match movement {
            Movement::Home => 0,
            Movement::End => self.text.len(),
            Movement::Left if !extend && !selection.is_empty() => selection.start,
            Movement::Right if !extend && !selection.is_empty() => selection.end,
            Movement::Left => previous_boundary(&self.text, self.caret),
            Movement::Right => next_boundary(&self.text, self.caret),
            Movement::WordLeft => previous_word(
                &self.text,
                if extend { self.caret } else { selection.start },
            ),
            Movement::WordRight => {
                next_word(&self.text, if extend { self.caret } else { selection.end })
            }
        };
        if !extend {
            self.anchor = self.caret;
        }
        previous != (self.anchor, self.caret)
    }
    pub fn delete(&mut self, backward: bool) -> bool {
        if self.is_composing() {
            return false;
        }
        self.composition = None;
        let mut selection = self.selection();
        if selection.is_empty() {
            if backward {
                selection.start = previous_boundary(&self.text, self.caret);
            } else {
                selection.end = next_boundary(&self.text, self.caret);
            }
        }
        self.replace(selection, "")
    }
    fn replace(&mut self, selection: Range<usize>, text: &str) -> bool {
        // Match HTML single-line text fields: multiline paste/IME commits do
        // not introduce line breaks into filenames, paths or numeric fields.
        let normalized = if self.multiline {
            text.replace("\r\n", "\n").replace('\r', "\n")
        } else {
            text.replace(['\r', '\n'], "")
        };
        let text = normalized.as_str();
        let changed = self.text[selection.clone()] != *text || self.anchor != self.caret;
        self.text.replace_range(selection.clone(), text);
        // Inserting a combining sequence can merge across either boundary.
        // Keep the caret at the next valid extended-grapheme boundary.
        let end = selection.start + text.len();
        self.caret = if boundary(&self.text, end) {
            end
        } else {
            next_boundary(&self.text, end)
        };
        self.anchor = self.caret;
        changed
    }
}

fn boundary(text: &str, offset: usize) -> bool {
    offset == text.len()
        || text
            .grapheme_indices(true)
            .any(|(index, _)| index == offset)
}
fn previous_boundary(text: &str, offset: usize) -> usize {
    text.grapheme_indices(true)
        .map(|(index, _)| index)
        .take_while(|index| *index < offset)
        .last()
        .unwrap_or(0)
}
fn next_boundary(text: &str, offset: usize) -> usize {
    text.grapheme_indices(true)
        .map(|(index, _)| index)
        .find(|index| *index > offset)
        .unwrap_or(text.len())
}
fn previous_word(text: &str, offset: usize) -> usize {
    text.unicode_word_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index < offset)
        .filter(|index| boundary(text, *index))
        .last()
        .unwrap_or(0)
}
fn next_word(text: &str, offset: usize) -> usize {
    text.unicode_word_indices()
        .map(|(index, word)| index + word.len())
        .find(|index| *index > offset && boundary(text, *index))
        .unwrap_or(text.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiline_edits_keep_lines_and_vertical_selection_direction() {
        let mut edit = TextEdit::multiline("first\n中🙂last".into());
        assert!(edit.move_to(2, false));
        assert!(edit.move_to("first\n中".len(), true));
        assert_eq!(edit.selected_text(), "rst\n中");
        assert!(edit.move_to(1, true));
        assert_eq!(edit.selected_text(), "i");
        assert!(!edit.move_to(7, false), "Cannot split Chinese UTF-8");
        edit.select_all();
        edit.insert("one\r\ntwo\rthree\nfour");
        assert_eq!(edit.text(), "one\ntwo\nthree\nfour");
        edit.preedit("候选", Some(6..6));
        assert!(!edit.move_to(0, false));
        edit.commit("\n中文");
        assert!(edit.text().ends_with("\n中文"));
    }

    #[test]
    fn selection_rejects_split_utf8_and_combining_sequences() {
        let mut edit = TextEdit::new("中e\u{301}🙂".into());
        assert!(!edit.set_selection(1..3));
        assert!(!edit.set_selection(3..4));
        assert!(!edit.set_selection(99..99));
        assert!(!edit.set_selection(3..0));
        assert!(edit.set_selection(3..6));
        assert_eq!(edit.selected_text(), "e\u{301}");
        edit.insert("文");
        assert_eq!(edit.text(), "中文🙂");
        assert_eq!(edit.caret(), "中文".len());
    }

    #[test]
    fn backspace_and_delete_remove_whole_emoji_flags_and_combining_graphemes() {
        for cluster in ["e\u{301}", "👨‍👩‍👧‍👦", "🇨🇳", "👍🏽"] {
            let mut edit = TextEdit::new(format!("a{cluster}b"));
            edit.move_caret(Movement::Left, false);
            assert!(edit.delete(true));
            assert_eq!(edit.text(), "ab");
            let mut edit = TextEdit::new(format!("a{cluster}b"));
            edit.set_selection(1..1);
            assert!(edit.delete(false));
            assert_eq!(edit.text(), "ab");
        }
    }

    #[test]
    fn shift_movement_preserves_direction_and_collapses_selections() {
        let mut edit = TextEdit::new("a中🙂".into());
        edit.move_caret(Movement::Left, true);
        edit.move_caret(Movement::Left, true);
        assert_eq!(edit.selected_text(), "中🙂");
        assert_eq!(edit.caret(), 1);
        assert_eq!(edit.display_caret(), Some(1));
        edit.move_caret(Movement::Right, true);
        assert_eq!(edit.selected_text(), "🙂");
        edit.move_caret(Movement::Left, false);
        assert_eq!(edit.selection(), 4..4);
        edit.move_caret(Movement::Home, true);
        assert_eq!(edit.selected_text(), "a中");
        edit.move_caret(Movement::End, false);
        assert_eq!(edit.selection(), 8..8);
    }

    #[test]
    fn word_movement_uses_unicode_words_and_preserves_shift_anchor_and_graphemes() {
        let text = "cafe\u{301},  l’été 👨‍👩‍👧‍👦 שלום 中文";
        let mut edit = TextEdit::new(text.into());
        edit.move_caret(Movement::Home, false);
        edit.move_caret(Movement::WordRight, false);
        assert_eq!(edit.caret(), "cafe\u{301}".len());
        edit.move_caret(Movement::WordRight, true);
        assert_eq!(edit.selected_text(), ",  l’été");
        edit.move_caret(Movement::WordRight, true);
        assert_eq!(edit.selected_text(), ",  l’été 👨‍👩‍👧‍👦 שלום");
        edit.move_caret(Movement::WordLeft, true);
        assert_eq!(edit.selected_text(), ",  l’été 👨‍👩‍👧‍👦 ");
        assert!(boundary(text, edit.caret()));
        edit.move_caret(Movement::WordLeft, false);
        assert_eq!(edit.selection(), 0..0);
        edit.move_caret(Movement::End, false);
        edit.move_caret(Movement::WordLeft, true);
        assert_eq!(edit.selected_text(), "文");
        edit.move_caret(Movement::WordLeft, true);
        assert_eq!(edit.selected_text(), "中文");
        edit.move_caret(Movement::WordRight, false);
        assert_eq!(edit.selection(), text.len()..text.len());
        edit.preedit("候选", Some(6..6));
        assert!(!edit.move_caret(Movement::WordLeft, true));
        assert_eq!(edit.text(), text);
    }

    #[test]
    fn preedit_is_drawn_separately_and_commits_over_the_original_selection_once() {
        let mut edit = TextEdit::new("beforeOLDafter".into());
        edit.set_selection(6..9);
        edit.preedit("zhong", Some(0..5));
        edit.preedit("中", Some(3..3));
        assert_eq!(edit.text(), "beforeOLDafter");
        assert_eq!(edit.display_text(), "before中after");
        assert_eq!(edit.composition_range(), Some(6..9));
        assert_eq!(edit.display_selection(), Some(9..9));
        assert_eq!(edit.display_caret(), Some(9));
        assert!(!edit.move_caret(Movement::End, false));
        assert!(!edit.delete(true));
        edit.preedit("", None);
        assert!(
            !edit.is_composing(),
            "Empty preedit also signals cancellation"
        );
        assert_eq!(
            edit.display_text(),
            "beforeOLDafter",
            "An IME cancellation must reveal the original selection"
        );
        assert_eq!(edit.display_selection(), Some(6..9));
        assert_eq!(edit.composition_range(), None);
        edit.commit("中文");
        assert_eq!(edit.text(), "before中文after");
        assert_eq!(edit.selection(), 12..12);
        assert!(!edit.is_composing());
    }

    #[test]
    fn composition_cancel_preserves_committed_text_and_rejects_invalid_ime_offsets() {
        let mut edit = TextEdit::new("abc".into());
        edit.set_selection(1..2);
        assert!(!edit.preedit("中", Some(1..1)));
        edit.preedit("かな", None);
        assert_eq!(edit.display_selection(), None);
        assert!(edit.clear_preedit());
        assert_eq!(edit.text(), "abc");
        assert_eq!(edit.selected_text(), "b");
        edit.commit("é");
        assert_eq!(edit.text(), "aéc");
        assert_eq!(edit.selection(), 3..3);
    }

    #[test]
    fn clipboard_and_all_three_screenshot_field_values_share_replacement_semantics() {
        for (initial, pasted) in [
            ("~/Pictures", "/tmp/截图"),
            ("SnapLingo", "截图_🙂"),
            ("10000", "24000"),
        ] {
            let mut edit = TextEdit::new(initial.into());
            edit.select_all();
            let copied = edit.selected_text().to_string();
            assert_eq!(copied, initial);
            edit.insert(pasted);
            assert_eq!(edit.text(), pasted);
            assert_eq!(edit.selection(), pasted.len()..pasted.len());
        }
    }

    #[test]
    fn single_line_paste_and_ime_commit_remove_only_cr_and_lf() {
        let mut edit = TextEdit::new("prefix".into());
        edit.insert("\r\n中文\nname\r🙂");
        assert_eq!(edit.text(), "prefix中文name🙂");
        edit.select_all();
        edit.preedit("候選", Some(6..6));
        edit.commit("a\r\nb\rc\n\t");
        assert_eq!(edit.text(), "abc\t");
        assert_eq!(edit.selection(), 4..4);
    }
}
