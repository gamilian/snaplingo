use std::collections::{BTreeSet, HashMap};
use std::sync::OnceLock;

use ab_glyph::{FontArc, FontVec};
use fontdb::{Database, Family, Query, Stretch, Style, Weight};
use parking_lot::Mutex;

use snaplingo_core::application::capture::{AnnotationFont, AnnotationFontProvider};

#[derive(Default)]
pub(crate) struct SystemAnnotationFonts {
    database: OnceLock<Database>,
    cache: Mutex<HashMap<(String, bool, bool), Option<AnnotationFont>>>,
}

impl SystemAnnotationFonts {
    fn database(&self) -> &Database {
        self.database.get_or_init(|| {
            let mut database = Database::new();
            database.load_system_fonts();
            database
        })
    }
}

impl AnnotationFontProvider for SystemAnnotationFonts {
    fn families(&self) -> Vec<String> {
        self.database()
            .faces()
            .flat_map(|face| face.families.iter().map(|(family, _)| family.clone()))
            .filter(|name| !name.starts_with('@'))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn resolve(&self, family: &str, bold: bool, italic: bool) -> Option<AnnotationFont> {
        if family.is_empty() {
            return None;
        }
        let key = (family.to_owned(), bold, italic);
        if let Some(cached) = self.cache.lock().get(&key) {
            return cached.clone();
        }
        let database = self.database();
        let resolved = database
            .query(&Query {
                families: &[Family::Name(family)],
                weight: if bold { Weight::BOLD } else { Weight::NORMAL },
                style: if italic { Style::Italic } else { Style::Normal },
                stretch: Stretch::Normal,
            })
            .and_then(|id| {
                let face = database.face(id)?;
                let font = database
                    .with_face_data(id, |bytes, index| {
                        FontVec::try_from_vec_and_index(bytes.to_vec(), index)
                            .ok()
                            .map(FontArc::new)
                    })
                    .flatten()?;
                Some(AnnotationFont {
                    font,
                    bold: face.weight.0 >= 600,
                    italic: face.style != Style::Normal,
                })
            });
        self.cache.lock().insert(key, resolved.clone());
        resolved
    }
}
