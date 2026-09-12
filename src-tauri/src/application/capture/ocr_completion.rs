use std::sync::OnceLock;

use regex::Regex;
use serde::Deserialize;

use crate::domain::capture::CaptureSessionId;
use crate::domain::{OcrRequest, OcrSettings};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CaptureOcrTarget {
    OcrWindow,
    TranslationWindow,
    Clipboard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CaptureOcrStatus {
    Recognizing,
    Copied,
    Failed,
}

pub(super) struct PendingCaptureOcr {
    pub session_id: CaptureSessionId,
    pub generation: u64,
    pub request: OcrRequest,
    pub preview: Option<String>,
    pub target: CaptureOcrTarget,
    pub settings: OcrSettings,
    pub clipboard_revision: u64,
}

impl PendingCaptureOcr {
    pub fn shows_status(&self) -> bool {
        self.target != CaptureOcrTarget::Clipboard || !self.settings.hide_silent_status
    }
}

// Keep clipboard output consistent with applyOcrTextPreferences in ocrTextProcessing.ts.
// Result-window output is still formatted by that existing frontend function.
pub(super) fn clipboard_text(text: &str, settings: &OcrSettings) -> String {
    static HAN_SPACES: OnceLock<Regex> = OnceLock::new();
    let han_spaces =
        HAN_SPACES.get_or_init(|| Regex::new(r"(\p{Han})[\t \x{a0}]+(\p{Han})").unwrap());
    let mut text = text.replace("\r\n", "\n").replace('\r', "\n");
    if !settings.preserve_formatting {
        let mut lines: Vec<String> = Vec::new();
        for line in text.split('\n').map(str::trim) {
            if let Some(previous) = lines.last_mut().filter(|previous| !previous.is_empty()) {
                if !line.is_empty() && should_join(previous, line) {
                    if !token_continuation(previous)
                        && !previous.ends_with(is_cjk)
                        && !line.starts_with(is_cjk)
                    {
                        previous.push(' ');
                    }
                    previous.push_str(line);
                    continue;
                }
            }
            lines.push(line.into());
        }
        text = lines.join("\n");
    }
    if settings.remove_chinese_spaces {
        // Two passes also cover overlapping pairs in a run such as “你 好 世 界”.
        for _ in 0..2 {
            text = han_spaces.replace_all(&text, "$1$2").into_owned();
        }
    }
    if settings.preserve_formatting {
        text.trim().into()
    } else {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

fn token_continuation(previous: &str) -> bool {
    previous.ends_with([
        '@', '/', '.', '_', '~', ':', '?', '#', '&', '=', '%', '+', '-',
    ]) || (previous.contains('@') && !previous.contains(' '))
}

fn should_join(previous: &str, next: &str) -> bool {
    static LIST: OnceLock<Regex> = OnceLock::new();
    static LABEL: OnceLock<Regex> = OnceLock::new();
    static TOKEN: OnceLock<Regex> = OnceLock::new();
    let list = LIST.get_or_init(|| Regex::new(r"^(?:[-*•·]|[0-9]+[.)、])\s+").unwrap());
    let label = LABEL.get_or_init(|| {
        Regex::new(r"(?i)^(?:Email|E-mail|Phone|Tel|Mobile|URL|网址|邮箱|电话|手机|座机|验证码|订单号)(?:(?-u:\b)|\s|$)").unwrap()
    });
    let token = TOKEN.get_or_init(|| {
        Regex::new(r"(?i)(?:(?:https?://|www\.)\S+|[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}|[0-9][0-9\s().-]{6,}[0-9])$").unwrap()
    });
    if list.is_match(next) {
        return false;
    }
    if token_continuation(previous) {
        return true;
    }
    if token.is_match(previous) && (label.is_match(previous) || label.is_match(next)) {
        return false;
    }
    !previous.ends_with(['。', '！', '？', '!', '?', '；', ';'])
}

fn is_cjk(character: char) -> bool {
    matches!(character, '\u{3400}'..='\u{9fff}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Fixture {
        text: String,
        preserve_formatting: bool,
        remove_chinese_spaces: bool,
        expected: String,
    }

    #[test]
    fn clipboard_formatting_matches_result_window_preferences() {
        let fixtures: Vec<Fixture> = serde_json::from_str(include_str!(
            "../../../../src/utils/ocrTextPreferences.fixtures.json"
        ))
        .unwrap();
        for fixture in fixtures {
            let settings = OcrSettings {
                preserve_formatting: fixture.preserve_formatting,
                remove_chinese_spaces: fixture.remove_chinese_spaces,
                ..OcrSettings::default()
            };
            assert_eq!(
                clipboard_text(&fixture.text, &settings),
                fixture.expected,
                "{}",
                fixture.text
            );
        }
    }
}
