//! Single-line caret geometry uses the same shaping engine and fonts as paint.
use crate::foundation::{FontFamily, Point, TextLayout, TextMeasurement, TextMeasurer};
use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Weight, Wrap};
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Hash, PartialEq, Eq)]
struct Key {
    text: String,
    size: u32,
    family: u8,
    weight: u16,
}
pub struct RendererTextMetrics {
    fonts: FontSystem,
    cache: HashMap<Key, TextMeasurement>,
}
impl Default for RendererTextMetrics {
    fn default() -> Self {
        Self {
            fonts: super::system_fonts(),
            cache: HashMap::new(),
        }
    }
}
impl TextMeasurer for RendererTextMetrics {
    fn line_layout(
        &mut self,
        text: &str,
        width: f32,
        size: f32,
        weight: u16,
        font: FontFamily,
        line_height: f32,
    ) -> TextLayout {
        let mut buffer = Buffer::new(
            &mut self.fonts,
            Metrics::new(size.max(1.), line_height.max(1.)),
        );
        buffer.set_size(&mut self.fonts, Some(width.max(1.)), None);
        buffer.set_wrap(&mut self.fonts, Wrap::WordOrGlyph);
        let family = match font {
            FontFamily::Sans => Family::SansSerif,
            FontFamily::WindowsUi => Family::Name(if cfg!(target_os = "windows") {
                "Segoe UI"
            } else {
                "Arial"
            }),
            FontFamily::Serif => Family::Serif,
            FontFamily::Mono => Family::Monospace,
        };
        super::text::set_text(
            &mut buffer,
            &mut self.fonts,
            text,
            &Attrs::new().family(family).weight(Weight(weight)),
        );
        buffer.shape_until_scroll(&mut self.fonts, false);
        let mut starts = vec![0];
        for (index, ch) in text.char_indices() {
            if ch == '\n' {
                starts.push(index + 1);
            }
        }
        let mut positions = std::collections::BTreeMap::new();
        let mut height = line_height.max(1.);
        for run in buffer.layout_runs() {
            height = height.max(run.line_top + run.line_height);
            let offset = starts.get(run.line_i).copied().unwrap_or(text.len());
            let first = run.glyphs.first().map_or(0, |g| g.start);
            let last = run.glyphs.last().map_or(0, |g| g.end);
            for (index, _) in run
                .text
                .grapheme_indices(true)
                .chain(std::iter::once((run.text.len(), "")))
            {
                if index < first || index > last {
                    continue;
                }
                let x = if let Some(glyph) = run
                    .glyphs
                    .iter()
                    .find(|g| g.start <= index && index < g.end)
                {
                    let count = run.text[glyph.start..glyph.end]
                        .graphemes(true)
                        .count()
                        .max(1);
                    let before = run.text[glyph.start..index].graphemes(true).count();
                    let fraction = before as f32 / count as f32;
                    glyph.x
                        + glyph.w
                            * if glyph.level.is_rtl() {
                                1. - fraction
                            } else {
                                fraction
                            }
                } else {
                    run.glyphs
                        .last()
                        .map_or(0., |g| if g.level.is_rtl() { g.x } else { g.x + g.w })
                };
                positions.insert(offset + index, Point { x, y: run.line_top });
            }
        }
        TextLayout {
            height,
            caret_stops: positions.into_iter().collect(),
        }
    }
    fn measure(&mut self, text: &str, size: f32, weight: u16, font: FontFamily) -> TextMeasurement {
        let key = Key {
            text: text.into(),
            size: size.to_bits(),
            family: match font {
                FontFamily::Sans => 0,
                FontFamily::WindowsUi => 3,
                FontFamily::Serif => 1,
                FontFamily::Mono => 2,
            },
            weight,
        };
        let state = self;
        if let Some(cached) = state.cache.get(&key) {
            return cached.clone();
        }
        let mut buffer = Buffer::new(
            &mut state.fonts,
            Metrics::new(size.max(1.0), size.max(1.0) * 1.35),
        );
        buffer.set_wrap(&mut state.fonts, Wrap::None);
        buffer.set_size(&mut state.fonts, None, Some(size.max(1.0) * 2.0));
        let family = match font {
            FontFamily::Sans => Family::SansSerif,
            FontFamily::WindowsUi => Family::Name(if cfg!(target_os = "windows") {
                "Segoe UI"
            } else {
                "Arial"
            }),
            FontFamily::Serif => Family::Serif,
            FontFamily::Mono => Family::Monospace,
        };
        super::text::set_text(
            &mut buffer,
            &mut state.fonts,
            text,
            &Attrs::new().family(family).weight(Weight(weight)),
        );
        buffer.shape_until_scroll(&mut state.fonts, false);
        let boundaries: Vec<_> = text
            .grapheme_indices(true)
            .map(|(i, _)| i)
            .chain(std::iter::once(text.len()))
            .collect();
        let mut positions = Vec::with_capacity(boundaries.len());
        if let Some(run) = buffer.layout_runs().next() {
            for index in boundaries {
                let x = if let Some(glyph) = run
                    .glyphs
                    .iter()
                    .find(|glyph| glyph.start <= index && index < glyph.end)
                {
                    // A shaping cluster can contain multiple graphemes (e.g. fi
                    // ligatures). Distribute only at valid grapheme stops.
                    let cluster = &text[glyph.start..glyph.end];
                    let count = cluster.graphemes(true).count().max(1);
                    let before = text[glyph.start..index].graphemes(true).count();
                    let fraction = before as f32 / count as f32;
                    glyph.x
                        + glyph.w
                            * if glyph.level.is_rtl() {
                                1.0 - fraction
                            } else {
                                fraction
                            }
                } else if let Some(glyph) = run.glyphs.iter().find(|glyph| glyph.end == index) {
                    if glyph.level.is_rtl() {
                        glyph.x
                    } else {
                        glyph.x + glyph.w
                    }
                } else {
                    0.0
                };
                positions.push((index, x));
            }
        } else {
            positions.push((0, 0.0));
        }
        let measured = TextMeasurement {
            caret_stops: positions,
        };
        if state.cache.len() >= 256 {
            state.cache.clear();
        }
        state.cache.insert(key, measured.clone());
        measured
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_line_endings_keep_original_caret_bytes_without_extra_lines() {
        let mut metrics = RendererTextMetrics::default();
        for (text, lines) in [("中文\r\nnext", 2), ("中文\r\n\r\nnext", 3)] {
            let layout = metrics.line_layout(text, 300., 14., 500, FontFamily::Sans, 28.);
            assert_eq!(layout.height, lines as f32 * 28.);
            let offset = text.find("next").unwrap();
            assert_eq!(
                layout
                    .caret_stops
                    .iter()
                    .find(|(byte, _)| *byte == offset)
                    .unwrap()
                    .1,
                Point {
                    x: 0.,
                    y: (lines - 1) as f32 * 28.
                }
            );
            assert_eq!(layout.caret_stops.last().unwrap().0, text.len());
            assert!(layout
                .caret_stops
                .iter()
                .all(|(byte, _)| text.is_char_boundary(*byte)));
        }
    }

    #[test]
    fn wrapped_multiline_layout_keeps_caret_bytes_and_requested_line_height() {
        let mut metrics = RendererTextMetrics::default();
        let text = "中文 e\u{301}🙂 wraps across lines\nnext";
        let layout = metrics.line_layout(text, 75., 14., 400, FontFamily::Sans, 28.);
        assert!(layout.height >= 84.);
        assert_eq!(
            layout
                .caret_stops
                .iter()
                .map(|(b, _)| *b)
                .collect::<Vec<_>>(),
            text.grapheme_indices(true)
                .map(|(b, _)| b)
                .chain(std::iter::once(text.len()))
                .collect::<Vec<_>>()
        );
        assert!(layout
            .caret_stops
            .iter()
            .all(|(_, p)| p.x.is_finite() && p.y.is_finite()));
        assert!(layout.caret_stops.iter().any(|(_, p)| p.y >= 56.));
        let next = layout
            .caret_stops
            .iter()
            .find(|(b, _)| *b == text.find("next").unwrap())
            .unwrap()
            .1;
        assert_eq!(next.x, 0.);
        assert_eq!(next.y % 28., 0.);
    }
    #[test]
    fn caret_positions_use_shaped_proportional_text_and_grapheme_boundaries() {
        let mut metrics = RendererTextMetrics::default();
        let narrow = metrics.measure("iii", 14.0, 400, FontFamily::Sans);
        let wide = metrics.measure("WWW", 14.0, 400, FontFamily::Sans);
        assert!(wide.caret_stops.last().unwrap().1 > narrow.caret_stops.last().unwrap().1 * 1.5);
        let text = "中e\u{301}👩‍💻文";
        let result = metrics.measure(text, 14.0, 400, FontFamily::Sans);
        assert_eq!(
            result
                .caret_stops
                .iter()
                .map(|(i, _)| *i)
                .collect::<Vec<_>>(),
            text.grapheme_indices(true)
                .map(|(i, _)| i)
                .chain(std::iter::once(text.len()))
                .collect::<Vec<_>>()
        );
        assert!(result.caret_stops.iter().all(|(_, x)| x.is_finite()));
    }
}
