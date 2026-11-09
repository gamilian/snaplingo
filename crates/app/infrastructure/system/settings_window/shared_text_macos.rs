//! AppKit's UTF-16 text services feed the same Rust editor used on Windows.
use std::ops::Range;

use objc2::{
    rc::Retained,
    runtime::{AnyObject, Sel},
    sel, AnyThread, DefinedClass,
};
use objc2_app_kit::NSEvent;
use objc2_foundation::{
    NSArray, NSAttributedString, NSNotFound, NSPoint, NSRange, NSRect, NSSize, NSString,
};
use snaplingo_ui::{Event, Key, Modifiers, TextEdit};

use super::shared_view_macos::SharedFeatureView;

fn absent() -> NSRange {
    NSRange::new(NSNotFound as usize, 0)
}

fn byte_offset(text: &str, utf16: usize) -> Option<usize> {
    let mut units = 0;
    for (byte, ch) in text.char_indices() {
        if units == utf16 {
            return Some(byte);
        }
        units += ch.len_utf16();
    }
    (units == utf16).then_some(text.len())
}
fn byte_range(text: &str, range: NSRange) -> Option<Range<usize>> {
    Some(
        byte_offset(text, range.location)?
            ..byte_offset(text, range.location.checked_add(range.length)?)?,
    )
}
fn native_range(text: &str, range: Range<usize>) -> NSRange {
    NSRange::new(
        text[..range.start].encode_utf16().count(),
        text[range].encode_utf16().count(),
    )
}
fn plain_text(text: &AnyObject) -> Option<String> {
    text.downcast_ref::<NSString>()
        .map(ToString::to_string)
        .or_else(|| {
            text.downcast_ref::<NSAttributedString>()
                .map(|s| s.string().to_string())
        })
}
fn replacement(
    edit: &TextEdit,
    range: NSRange,
    text: &str,
) -> Option<(Range<usize>, String, usize)> {
    let display = edit.display_text();
    let requested = byte_range(&display, range)?;
    let Some(marked) = edit.composition_range() else {
        return Some((requested, text.into(), 0));
    };
    // Include the whole marked span, retaining every displayed byte outside
    // the native replacement. A partial marked replacement must not drop the
    // unselected prefix/suffix of an IME draft.
    let span = requested.start.min(marked.start)..requested.end.max(marked.end);
    let committed = edit.selection();
    let map = |byte| {
        if byte <= marked.start {
            byte
        } else {
            committed.end + byte - marked.end
        }
    };
    let prefix = &display[span.start..requested.start];
    let value = format!("{prefix}{text}{}", &display[requested.end..span.end]);
    Some((map(span.start)..map(span.end), value, prefix.len()))
}

impl SharedFeatureView {
    pub(super) fn text_key_equivalent(&self, event: &NSEvent) -> bool {
        if self.ivars().ui.borrow().text_edit().is_none()
            || !event
                .modifierFlags()
                .contains(objc2_app_kit::NSEventModifierFlags::Command)
        {
            return false;
        }
        let Some(character) = event
            .charactersIgnoringModifiers()
            .and_then(|s| s.to_string().chars().next())
        else {
            return false;
        };
        if matches!(character, 'a' | 'A' | 'c' | 'C' | 'x' | 'X' | 'v' | 'V') {
            self.text_shortcut(character);
            true
        } else {
            false
        }
    }
    pub(super) fn text_key_down(&self, event: &NSEvent) {
        let editing = self.ivars().ui.borrow().text_edit().is_some();
        if editing {
            let flags = event.modifierFlags();
            let modifiers = Modifiers {
                shift: flags.contains(objc2_app_kit::NSEventModifierFlags::Shift),
                alt: flags.contains(objc2_app_kit::NSEventModifierFlags::Option),
                control: flags.contains(objc2_app_kit::NSEventModifierFlags::Control),
                meta: flags.contains(objc2_app_kit::NSEventModifierFlags::Command),
            };
            self.ivars().key_modifiers.set(modifiers);
            self.ivars().ime_committed.set(false);
            self.ivars().ime_was_composing.set(
                self.ivars()
                    .ui
                    .borrow()
                    .text_edit()
                    .is_some_and(|edit| edit.is_composing()),
            );
            if modifiers.meta && matches!(event.keyCode(), 123 | 124) {
                self.input(Event::KeyDown {
                    key: if event.keyCode() == 123 {
                        Key::Home
                    } else {
                        Key::End
                    },
                    modifiers: Modifiers {
                        shift: modifiers.shift,
                        ..Modifiers::default()
                    },
                    repeat: event.isARepeat(),
                });
                return;
            }
            if !modifiers.meta {
                self.interpretKeyEvents(&NSArray::from_slice(&[event]));
                return;
            }
        }
        self.key(event, true);
    }
    pub(super) fn text_shortcut(&self, character: char) {
        self.input(Event::KeyDown {
            key: Key::Character(character),
            modifiers: Modifiers {
                meta: true,
                ..Modifiers::default()
            },
            repeat: false,
        });
    }
    pub(super) fn ime_command(&self, command: Sel) {
        if self.ivars().ime_committed.get() {
            return;
        }
        let key = if command == sel!(insertNewline:) {
            Key::Enter
        } else if command == sel!(insertTab:) || command == sel!(insertBacktab:) {
            Key::Tab
        } else if command == sel!(cancelOperation:) {
            Key::Escape
        } else if command == sel!(deleteBackward:) {
            Key::Backspace
        } else if command == sel!(deleteForward:) {
            Key::Delete
        } else if command == sel!(moveUp:) || command == sel!(moveUpAndModifySelection:) {
            Key::ArrowUp
        } else if command == sel!(moveDown:) || command == sel!(moveDownAndModifySelection:) {
            Key::ArrowDown
        } else if command == sel!(moveLeft:)
            || command == sel!(moveLeftAndModifySelection:)
            || command == sel!(moveWordLeft:)
            || command == sel!(moveWordLeftAndModifySelection:)
        {
            Key::ArrowLeft
        } else if command == sel!(moveRight:)
            || command == sel!(moveRightAndModifySelection:)
            || command == sel!(moveWordRight:)
            || command == sel!(moveWordRightAndModifySelection:)
        {
            Key::ArrowRight
        } else if command == sel!(moveToBeginningOfLine:)
            || command == sel!(moveToBeginningOfLineAndModifySelection:)
        {
            Key::Home
        } else if command == sel!(moveToEndOfLine:)
            || command == sel!(moveToEndOfLineAndModifySelection:)
        {
            Key::End
        } else {
            return;
        };
        let composing = self
            .ivars()
            .ui
            .borrow()
            .text_edit()
            .is_some_and(|edit| edit.is_composing());
        if composing {
            if key == Key::Escape {
                self.cancel_composition();
            }
            return;
        }
        self.input(Event::KeyDown {
            key,
            modifiers: self.ivars().key_modifiers.get(),
            repeat: false,
        });
    }
    pub(crate) fn cancel_composition(&self) {
        self.input(Event::ImeDisabled);
        if let Some(context) = self.inputContext() {
            context.discardMarkedText();
        }
    }
    pub(super) fn ime_unmark(&self) {
        let draft = {
            let ui = self.ivars().ui.borrow();
            ui.text_edit().and_then(|edit| {
                edit.composition_range()
                    .map(|range| edit.display_text()[range].to_string())
            })
        };
        if let Some(draft) = draft {
            self.ivars().ime_committed.set(true);
            self.input(Event::ImeCommit(draft));
        }
    }
    fn replace_range(&self, range: NSRange, text: String) -> Option<(String, usize)> {
        if range.location == NSNotFound as usize {
            return Some((text, 0));
        }
        let (selection, text, offset) = {
            let ui = self.ivars().ui.borrow();
            replacement(ui.text_edit()?, range, &text)?
        };
        self.input(Event::TextSelection(selection.clone()));
        self.ivars()
            .ui
            .borrow()
            .text_edit()
            .is_some_and(|edit| edit.selection() == selection)
            .then_some((text, offset))
    }
    pub(super) fn ime_insert(&self, text: &AnyObject, replacement: NSRange) {
        if let Some(text) = plain_text(text) {
            let composing = self
                .ivars()
                .ui
                .borrow()
                .text_edit()
                .is_some_and(|edit| edit.is_composing());
            if let Some((text, _)) = self.replace_range(replacement, text) {
                if composing || self.ivars().ime_was_composing.get() {
                    self.ivars().ime_committed.set(true);
                }
                self.input(Event::ImeCommit(text));
            }
        }
    }
    pub(super) fn ime_mark(&self, text: &AnyObject, selection: NSRange, replacement: NSRange) {
        let Some(text) = plain_text(text) else {
            return;
        };
        let selection = if selection.location == NSNotFound as usize {
            None
        } else {
            let length = text.encode_utf16().count();
            let start = selection.location.min(length);
            let end = selection
                .location
                .saturating_add(selection.length)
                .min(length);
            let Some(range) = byte_range(&text, NSRange::new(start, end - start)) else {
                return;
            };
            Some(range)
        };
        if let Some((text, offset)) = self.replace_range(replacement, text) {
            self.input(Event::ImePreedit {
                text,
                selection: selection.map(|r| r.start + offset..r.end + offset),
            });
        }
    }
    pub(super) fn ime_marked_range(&self) -> NSRange {
        let ui = self.ivars().ui.borrow();
        ui.text_edit()
            .and_then(|edit| {
                edit.composition_range()
                    .map(|r| native_range(&edit.display_text(), r))
            })
            .unwrap_or_else(absent)
    }
    pub(super) fn ime_selected_range(&self) -> NSRange {
        let ui = self.ivars().ui.borrow();
        ui.text_edit()
            .and_then(|edit| {
                edit.display_selection()
                    .map(|r| native_range(&edit.display_text(), r))
            })
            .unwrap_or_else(absent)
    }
    pub(super) unsafe fn ime_substring(
        &self,
        range: NSRange,
        actual: *mut NSRange,
    ) -> Option<Retained<NSAttributedString>> {
        if !actual.is_null() {
            unsafe {
                *actual = absent();
            }
        }
        let ui = self.ivars().ui.borrow();
        let text = ui.text_edit()?.display_text();
        let bytes = byte_range(&text, range)?;
        if !actual.is_null() {
            unsafe {
                *actual = range;
            }
        }
        Some(NSAttributedString::initWithString(
            NSAttributedString::alloc(),
            &NSString::from_str(&text[bytes]),
        ))
    }
    pub(super) unsafe fn ime_first_rect(&self, _: NSRange, actual: *mut NSRange) -> NSRect {
        if !actual.is_null() {
            let ui = self.ivars().ui.borrow();
            let range = ui
                .text_edit()
                .and_then(|edit| {
                    edit.display_caret()
                        .map(|caret| native_range(&edit.display_text(), caret..caret))
                })
                .unwrap_or_else(absent);
            unsafe {
                *actual = range;
            }
        }
        let Some(window) = self.window() else {
            return NSRect::ZERO;
        };
        let scene = self.ivars().scene.borrow();
        let Some(scene) = scene.as_ref() else {
            return NSRect::ZERO;
        };
        let Some(caret) = scene.ime_cursor else {
            return NSRect::ZERO;
        };
        let origin = self.presentation_region(scene);
        let rect = NSRect::new(
            NSPoint::new((caret.x - origin.x) as f64, (caret.y - origin.y) as f64),
            NSSize::new(caret.width as f64, caret.height as f64),
        );
        window.convertRectToScreen(self.convertRect_toView(rect, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_ranges_round_trip_utf16_without_splitting_surrogates() {
        let text = "a中🙂e\u{301}";
        for (byte, _) in text
            .char_indices()
            .chain(std::iter::once((text.len(), '\0')))
        {
            let native = native_range(text, byte..byte);
            assert_eq!(byte_range(text, native), Some(byte..byte));
        }
        assert_eq!(byte_range(text, NSRange::new(2, 2)), Some(4..8));
        assert_eq!(byte_range(text, NSRange::new(3, 0)), None);
        assert_eq!(byte_range(text, absent()), None);
        assert_eq!(byte_range(text, NSRange::new(usize::MAX, 1)), None);
    }
    #[test]
    fn native_replacement_preserves_the_unselected_part_of_marked_text() {
        let mut edit = TextEdit::new("abc".into());
        edit.set_selection(1..1);
        edit.preedit("xy", Some(2..2));
        let (range, text, offset) = replacement(&edit, NSRange::new(2, 1), "Z").unwrap();
        assert_eq!(
            (range.clone(), text.clone(), offset),
            (1..1, "xZ".into(), 1)
        );
        edit.set_selection(range);
        edit.commit(&text);
        assert_eq!(edit.text(), "axZbc");
    }
}
