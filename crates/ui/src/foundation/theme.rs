#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    Native,
    Studio,
    Paper,
}
use super::Color;
#[derive(Clone, Copy)]
pub struct Palette {
    pub canvas: Color,
    pub surface: Color,
    pub sunken: Color,
    pub sidebar: Color,
    pub raised: Color,
    pub line: Color,
    pub line_strong: Color,
    pub ink: Color,
    pub muted: Color,
    pub faint: Color,
    pub accent_hover: Color,
    pub accent_line: Color,
    pub accent: Color,
    pub accent_soft: Color,
    pub accent_on: Color,
    pub good: Color,
    pub good_soft: Color,
    pub danger: Color,
}
pub fn palette(theme: Theme, dark: bool) -> Palette {
    const PALETTES: [Palette; 6] = [
        Palette {
            canvas: [248, 249, 252, 255],
            surface: [255, 255, 255, 255],
            sunken: [245, 247, 250, 255],
            sidebar: [241, 244, 248, 255],
            raised: [237, 241, 247, 255],
            line: [229, 233, 240, 255],
            line_strong: [212, 220, 231, 255],
            ink: [41, 53, 71, 255],
            muted: [109, 120, 137, 255],
            faint: [138, 148, 163, 255],
            accent: [57, 117, 220, 255],
            accent_hover: [41, 100, 200, 255],
            accent_line: [196, 214, 244, 255],
            accent_soft: [234, 241, 253, 255],
            accent_on: [255, 255, 255, 255],
            good: [60, 135, 107, 255],
            good_soft: [234, 245, 239, 255],
            danger: [200, 98, 97, 255],
        },
        Palette {
            canvas: [32, 39, 49, 255],
            surface: [41, 50, 62, 255],
            sunken: [48, 59, 73, 255],
            sidebar: [29, 37, 47, 255],
            raised: [58, 70, 85, 255],
            line: [59, 70, 83, 255],
            line_strong: [77, 91, 107, 255],
            ink: [227, 233, 241, 255],
            muted: [160, 174, 192, 255],
            faint: [134, 147, 165, 255],
            accent: [129, 171, 239, 255],
            accent_hover: [153, 191, 250, 255],
            accent_line: [67, 86, 115, 255],
            accent_soft: [48, 63, 85, 255],
            accent_on: [27, 44, 69, 255],
            good: [138, 189, 164, 255],
            good_soft: [44, 64, 56, 255],
            danger: [225, 145, 141, 255],
        },
        Palette {
            canvas: [243, 242, 239, 255],
            surface: [255, 254, 250, 255],
            sunken: [238, 234, 227, 255],
            sidebar: [233, 232, 228, 255],
            raised: [229, 224, 215, 255],
            line: [224, 220, 212, 255],
            line_strong: [207, 200, 186, 255],
            ink: [59, 57, 51, 255],
            muted: [128, 120, 109, 255],
            faint: [147, 139, 127, 255],
            accent: [156, 109, 54, 255],
            accent_hover: [128, 88, 44, 255],
            accent_line: [225, 210, 191, 255],
            accent_soft: [244, 232, 217, 255],
            accent_on: [255, 255, 255, 255],
            good: [84, 127, 101, 255],
            good_soft: [232, 240, 232, 255],
            danger: [179, 101, 96, 255],
        },
        Palette {
            canvas: [32, 35, 38, 255],
            surface: [37, 41, 44, 255],
            sunken: [43, 48, 52, 255],
            sidebar: [25, 29, 32, 255],
            raised: [52, 59, 64, 255],
            line: [53, 60, 64, 255],
            line_strong: [70, 79, 85, 255],
            ink: [224, 228, 229, 255],
            muted: [165, 175, 179, 255],
            faint: [129, 144, 151, 255],
            accent: [215, 167, 109, 255],
            accent_hover: [226, 184, 129, 255],
            accent_line: [90, 79, 63, 255],
            accent_soft: [59, 51, 42, 255],
            accent_on: [41, 34, 27, 255],
            good: [145, 185, 160, 255],
            good_soft: [39, 57, 49, 255],
            danger: [220, 146, 144, 255],
        },
        Palette {
            canvas: [247, 247, 239, 255],
            surface: [253, 252, 247, 255],
            sunken: [238, 240, 229, 255],
            sidebar: [240, 242, 232, 255],
            raised: [232, 237, 223, 255],
            line: [224, 228, 214, 255],
            line_strong: [204, 212, 193, 255],
            ink: [57, 74, 60, 255],
            muted: [121, 132, 110, 255],
            faint: [147, 155, 135, 255],
            accent: [83, 115, 92, 255],
            accent_hover: [66, 100, 76, 255],
            accent_line: [202, 211, 200, 255],
            accent_soft: [232, 238, 222, 255],
            accent_on: [255, 253, 248, 255],
            good: [100, 133, 106, 255],
            good_soft: [234, 240, 224, 255],
            danger: [188, 116, 100, 255],
        },
        Palette {
            canvas: [34, 43, 37, 255],
            surface: [41, 52, 43, 255],
            sunken: [48, 60, 49, 255],
            sidebar: [33, 41, 33, 255],
            raised: [59, 73, 58, 255],
            line: [61, 73, 59, 255],
            line_strong: [83, 99, 79, 255],
            ink: [229, 231, 216, 255],
            muted: [173, 182, 158, 255],
            faint: [141, 155, 130, 255],
            accent: [168, 191, 145, 255],
            accent_hover: [189, 207, 166, 255],
            accent_line: [79, 94, 74, 255],
            accent_soft: [60, 74, 53, 255],
            accent_on: [40, 52, 31, 255],
            good: [172, 191, 149, 255],
            good_soft: [54, 67, 46, 255],
            danger: [213, 154, 135, 255],
        },
    ];
    PALETTES[match theme {
        Theme::Native => 0,
        Theme::Studio => 2,
        Theme::Paper => 4,
    } + usize::from(dark)]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_colors_match_the_actual_previous_ui_theme_sources() {
        let base = include_str!("../../../../reference/web/styles/index.css");
        let themes = include_str!("../../../../reference/web/styles/themes.css");
        let cases = [
            (Theme::Native, false, base, ":root"),
            (Theme::Native, true, base, ":root[data-theme='dark']"),
            (
                Theme::Studio,
                false,
                themes,
                ":root[data-interface-theme='studio'][data-theme='light']",
            ),
            (
                Theme::Studio,
                true,
                themes,
                ":root[data-interface-theme='studio'][data-theme='dark']",
            ),
            (
                Theme::Paper,
                false,
                themes,
                ":root[data-interface-theme='paper'][data-theme='light']",
            ),
            (
                Theme::Paper,
                true,
                themes,
                ":root[data-interface-theme='paper'][data-theme='dark']",
            ),
        ];
        for (theme, dark, css, selector) in cases {
            let body = css
                .split_once(&format!("{selector} {{"))
                .unwrap()
                .1
                .split_once('}')
                .unwrap()
                .0;
            let p = palette(theme, dark);
            for (token, actual) in [
                ("canvas", p.canvas),
                ("surface", p.surface),
                ("sunken", p.sunken),
                ("sidebar", p.sidebar),
                ("raised", p.raised),
                ("line", p.line),
                ("line-strong", p.line_strong),
                ("ink", p.ink),
                ("muted", p.muted),
                ("faint", p.faint),
                ("accent", p.accent),
                ("accent-hover", p.accent_hover),
                ("accent-line", p.accent_line),
                ("accent-soft", p.accent_soft),
                ("accent-on", p.accent_on),
                ("good", p.good),
                ("good-soft", p.good_soft),
                ("danger", p.danger),
            ] {
                let prefix = format!("--{token}:");
                let rgb = body
                    .lines()
                    .find_map(|line| line.trim().strip_prefix(&prefix))
                    .unwrap()
                    .trim_end_matches(';')
                    .split_whitespace()
                    .map(|s| s.parse::<u8>().unwrap())
                    .collect::<Vec<_>>();
                assert_eq!(
                    actual,
                    [rgb[0], rgb[1], rgb[2], 255],
                    "{theme:?} dark={dark}, token {token}"
                );
            }
        }
    }
}
