use cosmic_text::{
    fontdb::Query, Attrs, AttrsList, Buffer, BufferLine, Fallback, Family, FontSystem, LineEnding,
    LineIter, PlatformFallback, Scroll, Shaping,
};
use std::collections::HashMap;
use unicode_script::{Script, UnicodeScript};
use unicode_segmentation::UnicodeSegmentation;

/// Keep CJK fallback consistent between paint and measurement. cosmic-text 0.14
/// only considers exact font weights in its preferred fallback families, so a
/// medium label can skip the bundled regular face and mix unrelated system fonts.
pub(super) fn set_text(buffer: &mut Buffer, fonts: &mut FontSystem, text: &str, attrs: &Attrs<'_>) {
    let primary = fonts
        .db()
        .query(&Query {
            families: &[attrs.family],
            weight: attrs.weight,
            stretch: attrs.stretch,
            style: attrs.style,
        })
        .and_then(|id| fonts.get_font(id));
    let mut choices = HashMap::new();
    // Use the same line parser as Buffer::set_text. set_rich_text uses bidi
    // paragraphs instead, splitting CRLF and discarding original line endings.
    buffer.lines.clear();
    for (range, ending) in LineIter::new(text) {
        let line = &text[range];
        let mut attributes = AttrsList::new(attrs);
        for (offset, grapheme) in line.grapheme_indices(true) {
            let mut selected = attrs.clone();
            let cjk = grapheme.chars().find(|ch| {
                matches!(
                    ch.script(),
                    Script::Han
                        | Script::Hiragana
                        | Script::Katakana
                        | Script::Hangul
                        | Script::Bopomofo
                )
            });
            if let Some(ch) = cjk.filter(|ch| {
                primary
                    .as_ref()
                    .is_none_or(|font| font.rustybuzz().glyph_index(*ch).is_none())
            }) {
                let candidates = choices.entry(ch.script()).or_insert_with(|| {
                    PlatformFallback
                        .script_fallback(ch.script(), fonts.locale())
                        .iter()
                        .copied()
                        .chain(std::iter::once("Noto Sans CJK SC"))
                        .filter_map(|family| {
                            let id = fonts.db().query(&Query {
                                families: &[Family::Name(family)],
                                weight: attrs.weight,
                                stretch: attrs.stretch,
                                style: attrs.style,
                            })?;
                            let weight = fonts.db().face(id)?.weight;
                            Some((family, weight, fonts.get_font(id)?))
                        })
                        .collect::<Vec<_>>()
                });
                if let Some((family, weight, _)) = candidates
                    .iter()
                    .find(|(_, _, font)| font.rustybuzz().glyph_index(ch).is_some())
                {
                    selected = selected.family(Family::Name(family)).weight(*weight);
                }
            }
            if selected != *attrs {
                attributes.add_span(offset..offset + grapheme.len(), &selected);
            }
        }
        buffer
            .lines
            .push(BufferLine::new(line, ending, attributes, Shaping::Advanced));
    }
    if buffer.lines.is_empty() {
        buffer.lines.push(BufferLine::new(
            "",
            LineEnding::default(),
            AttrsList::new(attrs),
            Shaping::Advanced,
        ));
    }
    buffer.set_scroll(Scroll::default());
    buffer.shape_until_scroll(fonts, false);
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmic_text::{Metrics, Weight, Wrap};

    #[test]
    fn fallback_preserves_plain_text_lines_endings_and_glyph_offsets() {
        let mut fonts = super::super::system_fonts();
        let attrs = Attrs::new().family(Family::SansSerif);
        for text in [
            "",
            "line\r\nnext",
            "a\r\n\r\nb",
            "a\rb",
            "a\n\rb",
            "a\nb\n",
            "a\u{2029}b",
            "中文\r\nnext",
            "a\r\n",
        ] {
            let mut plain = Buffer::new(&mut fonts, Metrics::new(12.5, 17.));
            plain.set_text(&mut fonts, text, &attrs, Shaping::Advanced);
            plain.shape_until_scroll(&mut fonts, false);
            let mut fallback = Buffer::new(&mut fonts, Metrics::new(12.5, 17.));
            set_text(&mut fallback, &mut fonts, text, &attrs);
            fallback.shape_until_scroll(&mut fonts, false);
            let lines = |buffer: &Buffer| {
                buffer
                    .lines
                    .iter()
                    .map(|line| (line.text().to_owned(), line.ending()))
                    .collect::<Vec<_>>()
            };
            assert_eq!(lines(&fallback), lines(&plain), "{text:?}");
            let offsets = |buffer: &Buffer| {
                buffer
                    .layout_runs()
                    .map(|run| {
                        (
                            run.line_i,
                            run.line_top,
                            run.line_height,
                            run.glyphs
                                .iter()
                                .map(|glyph| (glyph.start, glyph.end))
                                .collect::<Vec<_>>(),
                        )
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(offsets(&fallback), offsets(&plain), "{text:?}");
        }
    }

    #[test]
    fn missing_system_cjk_fonts_use_bundled_glyphs_without_changing_latin() {
        let (locale, mut db) = super::super::system_fonts().into_locale_and_db();
        let missing = PlatformFallback.script_fallback(Script::Han, &locale);
        let ids = db
            .faces()
            .filter(|face| {
                face.families.iter().any(|(family, _)| {
                    family != "Noto Sans CJK SC" && missing.contains(&family.as_str())
                })
            })
            .map(|face| face.id)
            .collect::<Vec<_>>();
        for id in ids {
            db.remove_face(id);
        }
        let mut fonts = FontSystem::new_with_locale_and_db(locale, db);
        for weight in [500, 600] {
            let attrs = Attrs::new()
                .family(Family::SansSerif)
                .weight(Weight(weight));
            let mut latin = Buffer::new(&mut fonts, Metrics::new(12.5, 17.));
            latin.set_text(&mut fonts, "Latin", &attrs, Shaping::Advanced);
            latin.shape_until_scroll(&mut fonts, false);
            let latin_fonts = latin
                .layout_runs()
                .flat_map(|run| {
                    run.glyphs
                        .iter()
                        .map(|glyph| (glyph.font_id, glyph.glyph_id, glyph.w))
                })
                .collect::<Vec<_>>();
            let mut buffer = Buffer::new(&mut fonts, Metrics::new(12.5, 17.));
            buffer.set_wrap(&mut fonts, Wrap::None);
            set_text(&mut buffer, &mut fonts, "智能选择 Latin 法律翻译", &attrs);
            buffer.shape_until_scroll(&mut fonts, false);
            let glyphs = buffer
                .layout_runs()
                .flat_map(|run| run.glyphs.iter())
                .collect::<Vec<_>>();
            let cjk = glyphs
                .iter()
                .filter(|glyph| glyph.start < 12 || glyph.start >= 19)
                .collect::<Vec<_>>();
            assert_eq!(cjk.len(), 8);
            for glyph in cjk {
                assert_ne!(glyph.glyph_id, 0);
                assert!((glyph.w - 12.5).abs() < 0.01);
                assert!(fonts
                    .db()
                    .face(glyph.font_id)
                    .unwrap()
                    .families
                    .iter()
                    .any(|(family, _)| family == "Noto Sans CJK SC"));
            }
            assert_eq!(
                glyphs
                    .iter()
                    .filter(|glyph| (13..18).contains(&glyph.start))
                    .map(|glyph| (glyph.font_id, glyph.glyph_id, glyph.w))
                    .collect::<Vec<_>>(),
                latin_fonts
            );
        }
    }
}
