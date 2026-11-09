//! Native surfaces read the same semantic color tokens as the previous UI.
use std::sync::OnceLock;

use objc2::{rc::Retained, MainThreadMarker};
use objc2_app_kit::{NSApplication, NSColor};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Palette {
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

fn read_palette(css: &str, selector: &str) -> Palette {
    let body = css
        .split_once(&format!("{selector} {{"))
        .unwrap()
        .1
        .split_once('}')
        .unwrap()
        .0;
    let token = |name: &str| {
        let prefix = format!("--{name}:");
        let channels = body
            .lines()
            .find_map(|line| line.trim().strip_prefix(&prefix))
            .unwrap()
            .trim_end_matches(';')
            .split_whitespace()
            .map(|channel| channel.parse::<u8>().unwrap())
            .collect::<Vec<_>>();
        channels.try_into().unwrap()
    };
    Palette {
        canvas: token("canvas"),
        sidebar: token("sidebar"),
        sunken: token("sunken"),
        surface: token("surface"),
        line: token("line"),
        ink: token("ink"),
        muted: token("muted"),
        faint: token("faint"),
        line_strong: token("line-strong"),
        good: token("good"),
        good_soft: token("good-soft"),
        accent_line: token("accent-line"),
        raised: token("raised"),
        accent: token("accent"),
        accent_hover: token("accent-hover"),
        accent_soft: token("accent-soft"),
        accent_on: token("accent-on"),
        danger: token("danger"),
    }
}

pub(super) fn palette(interface_theme: &str, dark: bool) -> Palette {
    static PALETTES: OnceLock<[Palette; 6]> = OnceLock::new();
    let colors = PALETTES.get_or_init(|| {
        let base = include_str!("../../../../../src/styles/index.css");
        let presets = include_str!("../../../../../src/styles/themes.css");
        [
            read_palette(base, ":root"),
            read_palette(base, ":root[data-theme='dark']"),
            read_palette(
                presets,
                ":root[data-interface-theme='studio'][data-theme='light']",
            ),
            read_palette(
                presets,
                ":root[data-interface-theme='studio'][data-theme='dark']",
            ),
            read_palette(
                presets,
                ":root[data-interface-theme='paper'][data-theme='light']",
            ),
            read_palette(
                presets,
                ":root[data-interface-theme='paper'][data-theme='dark']",
            ),
        ]
    });
    let offset = match interface_theme {
        "studio" => 2,
        "paper" => 4,
        _ => 0,
    };
    colors[offset + usize::from(dark)]
}

pub(super) fn color(channels: [u8; 3]) -> Retained<NSColor> {
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
