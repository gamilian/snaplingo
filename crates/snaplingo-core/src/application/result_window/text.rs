use crate::domain::OcrSettings;
use regex::Regex;
use std::sync::LazyLock;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OcrCopyToken {
    pub label: String,
    pub value: String,
}

pub struct TokenCandidate {
    pub label: String,
    pub value: String,
    pub start: usize,
    pub end: usize,
    pub priority: u8,
}

fn han(character: char) -> bool {
    static HAN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\p{Script=Han}$").unwrap());
    let mut utf8 = [0; 4];
    HAN.is_match(character.encode_utf8(&mut utf8))
}

fn cjk(character: char) -> bool {
    matches!(character as u32, 0x3400..=0x9fff)
}

pub fn normalize_ocr_text(text: &str, preferences: Option<&OcrSettings>) -> String {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let keep_format = preferences.is_some_and(|settings| settings.preserve_formatting);
    let mut result = if keep_format {
        text.trim().to_owned()
    } else {
        join_lines(&text)
    };
    if preferences.is_some_and(|settings| settings.remove_chinese_spaces) {
        let mut normalized = String::with_capacity(result.len());
        let mut characters = result.chars().peekable();
        let mut previous = None;
        while let Some(character) = characters.next() {
            if matches!(character, ' ' | '\t' | '\u{a0}') && previous.is_some_and(han) {
                let mut spaces = String::from(character);
                while characters
                    .peek()
                    .is_some_and(|c| matches!(c, ' ' | '\t' | '\u{a0}'))
                {
                    spaces.push(characters.next().unwrap());
                }
                if !characters.peek().copied().is_some_and(han) {
                    normalized.push_str(&spaces);
                }
            } else {
                normalized.push(character);
                previous = Some(character);
            }
        }
        result = normalized;
    }
    if preferences.is_some_and(|settings| !settings.preserve_formatting) {
        result = result.split_whitespace().collect::<Vec<_>>().join(" ");
    }
    result
}

fn continuation(previous: &str, next: &str) -> bool {
    previous.ends_with([
        '@', '/', '.', '_', '~', ':', '?', '#', '&', '=', '%', '+', '-',
    ]) || (previous.contains('@') && !previous.contains(' '))
        || (previous.ends_with('-') && next.starts_with(|c: char| c.is_ascii_digit()))
}

fn join_lines(text: &str) -> String {
    static LIST: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*(?:[-*•·]|\d+[.)、])\s+").unwrap());
    static LABEL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)^\s*(?:Email|E-mail|Phone|Tel|Mobile|URL|网址|邮箱|电话|手机|座机|验证码|订单号)(?:(?-u:\b)|\s|$)").unwrap()
    });
    static TOKEN_END: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)(?:(?:https?://|www\.)\S+|[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}|\d[\d\s().-]{6,}\d)$").unwrap()
    });
    let mut lines: Vec<String> = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() {
            if lines.last().is_none_or(|line| !line.is_empty()) {
                lines.push(String::new());
            }
            continue;
        }
        if let Some(previous) = lines.last_mut().filter(|line| !line.is_empty()) {
            let continues = continuation(previous, line);
            let joins = !LIST.is_match(line)
                && (continues
                    || !(TOKEN_END.is_match(previous)
                        && (LABEL.is_match(previous) || LABEL.is_match(line)))
                        && !previous.ends_with(['。', '！', '？', '!', '?', '；', ';']));
            if joins {
                if !continues
                    && !previous.chars().last().is_some_and(cjk)
                    && !line.chars().next().is_some_and(cjk)
                {
                    previous.push(' ');
                }
                previous.push_str(line);
                continue;
            }
        }
        lines.push(line.to_owned());
    }
    lines.join("\n").trim().to_owned()
}

pub fn copy_tokens(text: &str, mut candidates: Vec<TokenCandidate>) -> Vec<OcrCopyToken> {
    static RULES: LazyLock<Vec<(Regex, &str, u8)>> = LazyLock::new(|| {
        [
        (r"[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}", "邮箱", 100),
        (r"(?:https?://|www\.)[^\s<>\x{4e00}-\x{9fff}]+|\b(?:[A-Z0-9](?:[A-Z0-9-]*[A-Z0-9])?\.)+[A-Z]{2,}(?:/[^\s<>\x{4e00}-\x{9fff}]*)?", "网址", 98),
        (r"\b(?:\+?86[ .-]?)?1[3-9](?:[ .-]?\d){9}\b|\+\d{1,3}[ .-]*(?:\(\d{1,4}\)[ .-]*)?(?:\d[ .-]*){6,12}\d\b", "电话", 94),
        (r"\b\d{17}[\dXx]\b", "敏感信息", 95),
        (r"\b-?\d{1,2}\.\d{4,}\s*,\s*-?\d{1,3}\.\d{4,}\b", "坐标", 92),
        (r"\b\d{4}[-/.]\d{1,2}[-/.]\d{1,2}(?:\s+\d{1,2}:\d{2}(?::\d{2})?)?\b", "时间", 90),
        (r"(?:[¥￥$€£]\s*\d+(?:,\d{3})*(?:\.\d{1,2})?|\b\d+(?:,\d{3})*(?:\.\d{1,2})?\s*(?:元|人民币|USD|CNY|EUR|GBP)\b)", "金额", 88),
        (r"\b(?:(?:25[0-5]|2[0-4]\d|1?\d?\d)\.){3}(?:25[0-5]|2[0-4]\d|1?\d?\d)(?::\d{2,5})?\b", "地址", 80),
        (r"[\x{4e00}-\x{9fa5}][A-Z][A-Z0-9]{5,6}\b", "车牌", 78),
        (r"(?:航班|Flight)\s*([A-Z]{2}\d{3,4})\b", "航班", 76),
        (r"(?:车次|Train)\s*([GDCZTKY]\d{1,4})\b", "车次", 75),
        (r"(?:座位|Seat)\s*(\d{1,2}[A-Z])\b", "座位", 74),
        (r"@[A-Z0-9_][A-Z0-9_.-]{2,}", "账号", 72),
        (r"(?:Wi-?Fi|SSID|账号|用户名|User(?:name)?)[:：]\s*([A-Z0-9_.-]{3,})", "账号", 71),
        (r"(?:验证码|校验码|Code|OTP)[:：]?\s*([A-Z0-9]{4,8})\b", "验证码", 70),
        (r"(?:密码|Password|Passcode)[:：]?\s*([A-Z0-9_.@#$%&*!-]{4,})\b", "密码", 69),
        (r"\b[A-Z]{1,8}[-_]\d[A-Z0-9_-]{5,}\b", "编号", 60),
        (r"\b[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}\b", "编号", 55),
        (r"\b[A-Z0-9_-]{10,}\b", "编号", 50),
        (r"\b\d{6,}\b", "数字", 40),
    ].into_iter().map(|(pattern, label, priority)| {
        // Match the existing JavaScript rules' ASCII word boundaries around CJK text.
        let pattern = pattern.replace(r"\b", r"(?-u:\b)");
        (Regex::new(&format!("(?i){pattern}")).unwrap(), label, priority)
    }).collect()
    });
    for (pattern, label, priority) in RULES.iter() {
        for captures in pattern.captures_iter(text) {
            let matched = captures.get(1).unwrap_or_else(|| captures.get(0).unwrap());
            if *priority == 50 && !matched.as_str().contains(|c: char| c.is_ascii_digit()) {
                continue;
            }
            candidates.push(TokenCandidate {
                label: (*label).into(),
                value: matched.as_str().to_owned(),
                start: matched.start(),
                end: matched.end(),
                priority: *priority,
            });
        }
    }
    candidates.sort_by(|a, b| {
        a.start
            .cmp(&b.start)
            .then(b.priority.cmp(&a.priority))
            .then(b.end.cmp(&a.end))
    });
    let mut accepted: Vec<TokenCandidate> = Vec::new();
    for candidate in candidates {
        if !accepted
            .iter()
            .any(|old| candidate.start < old.end && candidate.end > old.start)
        {
            accepted.push(candidate);
        }
    }
    let mut seen = std::collections::HashSet::new();
    accepted
        .into_iter()
        .filter_map(|candidate| {
            let value = candidate
                .value
                .trim()
                .trim_start_matches(['(', '[', '{', '<', '"', '\'', '“', '‘'])
                .trim_end_matches([
                    ')', ']', '}', '>', ',', '"', '\'', '“', '”', '‘', '’', '。', '！', '？', '；',
                    '：', '、', '，', '.',
                ])
                .to_owned();
            if value.is_empty() || !seen.insert((candidate.label.clone(), value.clone())) {
                return None;
            }
            Some(OcrCopyToken {
                label: candidate.label,
                value,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ocr_text_keeps_format_or_joins_cjk_and_wrapped_urls() {
        let keep = OcrSettings::default();
        assert_eq!(
            normalize_ocr_text("你 好\r\n世界", Some(&keep)),
            "你好\n世界"
        );
        let flatten = OcrSettings {
            preserve_formatting: false,
            ..keep.clone()
        };
        assert_eq!(
            normalize_ocr_text("hello\nworld\n\n再 见", Some(&flatten)),
            "hello world 再见"
        );
        assert_eq!(
            normalize_ocr_text(
                "https://example.com/\npath\nEmail a@b.com\nPhone 13800138000",
                None
            ),
            "https://example.com/path\nEmail a@b.com\nPhone 13800138000"
        );
        assert_eq!(
            normalize_ocr_text("二 〇 二 六 · 漢 々", Some(&keep)),
            "二〇二六 · 漢々"
        );
        assert_eq!(normalize_ocr_text("𠮷\nhello", None), "𠮷 hello");
    }
    #[test]
    fn token_rules_keep_labels_and_do_not_split_addresses_dates_or_codes() {
        let tokens = copy_tokens(
            "OTP: AB1234 2026-11-05 09:30 地址 192.168.1.1:8080 航班 MU1234 座位 12A",
            vec![],
        );
        let values = tokens
            .iter()
            .map(|token| (token.label.as_str(), token.value.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            values,
            vec![
                ("验证码", "AB1234"),
                ("时间", "2026-11-05 09:30"),
                ("地址", "192.168.1.1:8080"),
                ("航班", "MU1234"),
                ("座位", "12A")
            ]
        );
    }
    #[test]
    fn detected_links_take_precedence_and_duplicates_are_removed() {
        let text = "https://host/AB_123456 @host";
        let tokens = copy_tokens(
            text,
            vec![TokenCandidate {
                label: "网址".into(),
                value: "https://host/AB_123456".into(),
                start: 0,
                end: 22,
                priority: 98,
            }],
        );
        assert_eq!(tokens[0].value, "https://host/AB_123456");
        assert!(!tokens.iter().any(|token| token.value == "AB_123456"));
    }
}

#[cfg(test)]
mod portable_contact_tests {
    use super::*;
    #[test]
    fn portable_contact_detection_keeps_the_same_fragments_without_a_platform_detector() {
        let text="📷 官网 https://example.com/docs?a=1，邮箱 support@example.com\n电话 138-0013-8000 或 +1 (650) 253-0000\n订单 AB-1234567890，验证码 123456\n预约时间 2026-11-09 14:30";
        let values = copy_tokens(text, vec![])
            .into_iter()
            .map(|token| (token.label, token.value))
            .collect::<Vec<_>>();
        assert_eq!(
            values,
            vec![
                ("网址".into(), "https://example.com/docs?a=1".into()),
                ("邮箱".into(), "support@example.com".into()),
                ("电话".into(), "138-0013-8000".into()),
                ("电话".into(), "+1 (650) 253-0000".into()),
                ("编号".into(), "AB-1234567890".into()),
                ("验证码".into(), "123456".into()),
                ("时间".into(), "2026-11-09 14:30".into())
            ]
        );
    }
}
