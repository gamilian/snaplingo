//! The two migrated feature pages share layout mechanics, not business state.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Field {
    General(GeneralField),
    Ocr(OcrField),
    Screenshot(ScreenshotField),
    Translation(TranslationField),
}
impl Field {
    pub fn label(self) -> &'static str {
        match self {
            Self::General(f) => f.label(),
            Self::Ocr(f) => f.label(),
            Self::Screenshot(f) => f.label(),
            Self::Translation(f) => f.label(),
        }
    }
    pub fn choices(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::General(f) => f.choices(),
            Self::Ocr(f) => f.choices(),
            Self::Screenshot(f) => f.choices(),
            Self::Translation(f) => f.choices(),
        }
    }
    pub fn value(self, s: &Snapshot) -> Value {
        match self {
            Self::General(f) => f.value(&s.general),
            Self::Ocr(f) => f.value(&s.ocr),
            Self::Screenshot(f) => f.value(&s.screenshot),
            Self::Translation(f) => f.value(&s.translation),
        }
    }
    pub fn edit(self, s: &mut Snapshot, value: Value) -> Option<Action> {
        if let Self::General(field) = self {
            return field.edit(s, value);
        }
        if let Self::Screenshot(field) = self {
            if field.value(&s.screenshot) == value
                && field != ScreenshotField::ResetAnnotationShortcuts
            {
                return None;
            }
            let mut next = s.screenshot.clone();
            field.apply(&mut next, value.clone()).ok()?;
            if !crate::settings::screenshot::valid_shortcuts(&next) {
                return None;
            }
            s.screenshot = next;
            return Some(Action::EditScreenshot { field, value });
        }
        match (&value, self.value(s)) {
            (Value::Toggle(_), Value::Toggle(_)) => {}
            (Value::Text(value), Value::Text(_))
                if self.choices().iter().any(|(id, _)| *id == value) => {}
            (Value::Number(value), Value::Number(_))
                if self
                    .range()
                    .is_some_and(|(min, max, _, _)| (min..=max).contains(value)) => {}
            _ => return None,
        }
        if self.value(s) == value {
            return None;
        }
        match self {
            Self::General(_) | Self::Screenshot(_) => unreachable!(),
            Self::Ocr(field) => {
                field.apply(&mut s.ocr, &value);
                Some(Action::EditOcr { field, value })
            }
            Self::Translation(field) => {
                field.apply(&mut s.translation, value.clone()).ok()?;
                Some(Action::EditTranslation { field, value })
            }
        }
    }
    pub fn target(self) -> Target {
        match self {
            Self::General(f) => Target::General(f),
            Self::Ocr(f) => Target::Ocr(f),
            Self::Screenshot(f) => Target::Screenshot(f),
            Self::Translation(f) => Target::Translation(f),
        }
    }
    pub fn from_target(target: Target) -> Option<Self> {
        match target {
            Target::General(f) => Some(Self::General(f)),
            Target::Ocr(f) => Some(Self::Ocr(f)),
            Target::Screenshot(f) => Some(Self::Screenshot(f)),
            Target::Translation(f) => Some(Self::Translation(f)),
            _ => None,
        }
    }
    pub fn option(self, index: usize) -> Target {
        match self {
            Self::General(f) => Target::GeneralOption(f, index),
            Self::Ocr(f) => Target::Option(f, index),
            Self::Screenshot(f) => Target::ScreenshotOption(f, index),
            Self::Translation(f) => Target::TranslationOption(f, index),
        }
    }
    pub fn range(self) -> Option<(u32, u32, u32, &'static str)> {
        match self {
            Self::General(field) => field.range(),
            Self::Screenshot(field) => field.range(),
            Self::Translation(TranslationField::MaxWindowHeightRatio) => Some((30, 90, 1, "%")),
            Self::Translation(TranslationField::WindowWidth) => Some((300, 1000, 10, "pt")),
            _ => None,
        }
    }
    pub fn group_start(self) -> bool {
        matches!(
            self,
            Self::Screenshot(
                ScreenshotField::AutoCopy
                    | ScreenshotField::RememberLastTool
                    | ScreenshotField::PinOpacity
            ) | Self::Ocr(OcrField::HideSilentStatus)
                | Self::Translation(
                    TranslationField::SelectionTextMode
                        | TranslationField::MaxWindowHeightRatio
                        | TranslationField::SelectionInputState
                        | TranslationField::WindowAlwaysOnTop
                )
        )
    }
    pub fn description(self) -> bool {
        self.description_text().is_some()
    }
    pub fn description_text(self) -> Option<&'static str> {
        match self {
            Self::Ocr(OcrField::GpuAcceleration) => {
                Some("用于 PaddleOCR；设备不支持时自动使用 CPU")
            }
            Self::Screenshot(field) => field.description(),
            _ => None,
        }
    }
    pub fn colors(self) -> &'static [Color] {
        match self {
            Self::Screenshot(field) => field.colors(),
            _ => &[],
        }
    }
    pub fn text_input(self) -> bool {
        if let Self::General(field) = self {
            return field.text_input();
        }
        matches!(
            self,
            Self::Screenshot(
                ScreenshotField::SavePath
                    | ScreenshotField::CustomFileName
                    | ScreenshotField::LongshotMaxHeight
            )
        )
    }
    pub fn text_size(self) -> f32 {
        if matches!(self, Self::Screenshot(_)) {
            10.5
        } else if self.range().is_some() {
            12.5
        } else {
            10.5
        }
    }
    pub fn local_shortcut(self) -> bool {
        matches!(
            self,
            Self::Screenshot(
                ScreenshotField::AnnotationShortcut(_) | ScreenshotField::PinBorderShortcut
            )
        )
    }
}
pub(super) fn fields(page: Page, section: Section, snapshot: &Snapshot) -> Vec<Field> {
    if page == Page::General {
        return crate::settings::general::fields(section, &snapshot.general)
            .into_iter()
            .map(Field::General)
            .collect();
    }
    if page == Page::Screenshot {
        return crate::settings::screenshot_view::fields(section, &snapshot.screenshot)
            .into_iter()
            .map(Field::Screenshot)
            .collect();
    }
    if page == Page::Translation {
        let fields = match section {
            Section::Language => &TranslationField::ALL[..3],
            Section::Window => &TranslationField::ALL[3..9],
            Section::Behavior => &TranslationField::ALL[9..],
            _ => &[],
        };
        fields.iter().map(|f| Field::Translation(*f)).collect()
    } else {
        let fields = match section {
            Section::Recognition => &OcrField::ALL[..5],
            Section::Window => &OcrField::ALL[5..],
            _ => &[],
        };
        fields.iter().map(|f| Field::Ocr(*f)).collect()
    }
}
