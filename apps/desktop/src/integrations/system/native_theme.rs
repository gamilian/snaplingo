//! Native surfaces read the same semantic color tokens as the previous UI.

use objc2::{rc::Retained, MainThreadMarker};
use objc2_app_kit::{NSApplication, NSColor};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Palette {
    pub canvas: [u8; 3],
    pub sidebar: [u8; 3],
    pub sunken: [u8; 3],
    pub surface: [u8; 3],
    pub line: [u8; 3],
    pub ink: [u8; 3],
    pub muted: [u8; 3],
    pub faint: [u8; 3],
    pub line_strong: [u8; 3],
    pub good: [u8; 3],
    pub good_soft: [u8; 3],
    pub accent_line: [u8; 3],
    pub raised: [u8; 3],
    pub accent: [u8; 3],
    pub accent_hover: [u8; 3],
    pub accent_soft: [u8; 3],
    pub accent_on: [u8; 3],
    pub danger: [u8; 3],
}

pub(crate) fn palette(interface_theme: &str, dark: bool) -> Palette {
    let theme = match interface_theme {
        "studio" => snaplingo_ui::Theme::Studio,
        "paper" => snaplingo_ui::Theme::Paper,
        _ => snaplingo_ui::Theme::Native,
    };
    let shared = snaplingo_ui::theme::palette(theme, dark);
    let rgb = |[r, g, b, _]: snaplingo_ui::Color| [r, g, b];
    Palette {
        canvas: rgb(shared.canvas),
        sidebar: rgb(shared.sidebar),
        sunken: rgb(shared.sunken),
        surface: rgb(shared.surface),
        line: rgb(shared.line),
        ink: rgb(shared.ink),
        muted: rgb(shared.muted),
        faint: rgb(shared.faint),
        line_strong: rgb(shared.line_strong),
        good: rgb(shared.good),
        good_soft: rgb(shared.good_soft),
        accent_line: rgb(shared.accent_line),
        raised: rgb(shared.raised),
        accent: rgb(shared.accent),
        accent_hover: rgb(shared.accent_hover),
        accent_soft: rgb(shared.accent_soft),
        accent_on: rgb(shared.accent_on),
        danger: rgb(shared.danger),
    }
}

pub(crate) fn color(channels: [u8; 3]) -> Retained<NSColor> {
    let [r, g, b] = channels.map(|value| f64::from(value) / 255.0);
    NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, 1.0)
}

pub(crate) fn is_dark(theme: &str) -> bool {
    match theme {
        "dark" => true,
        "light" => false,
        _ => NSApplication::sharedApplication(MainThreadMarker::new().unwrap())
            .effectiveAppearance()
            .name()
            .to_string()
            .contains("Dark"),
    }
}
