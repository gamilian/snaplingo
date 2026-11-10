#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyStyle {
    #[default]
    Mac,
    Windows,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub alt: bool,
    pub control: bool,
    pub meta: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Tab,
    Enter,
    Space,
    Escape,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
    Backspace,
    Delete,
    Character(char),
    Function(u8),
}
pub(crate) fn chord(key: Key, m: Modifiers, style: KeyStyle) -> Option<String> {
    let key = match key {
        Key::Character(c) if c.is_ascii_alphanumeric() => c.to_ascii_uppercase().to_string(),
        Key::Function(n) if (1..=24).contains(&n) => format!("F{n}"),
        _ => return None,
    };
    let mut out = String::new();
    for (on, glyph) in [
        (m.shift, '⇧'),
        (m.alt, '⌥'),
        (
            if style == KeyStyle::Windows {
                m.control
            } else {
                m.meta
            },
            '⌘',
        ),
        (
            if style == KeyStyle::Windows {
                m.meta
            } else {
                m.control
            },
            if style == KeyStyle::Windows {
                '⊞'
            } else {
                '⌃'
            },
        ),
    ] {
        if on {
            out.push(glyph);
        }
    }
    out.push_str(&key);
    Some(out)
}
