use objc2::rc::Retained;
use objc2_foundation::{NSDataDetector, NSMatchingOptions, NSRange, NSString, NSTextCheckingType};

use crate::application::result_window::{
    copy_tokens, normalize_ocr_text, OcrCopyToken, TokenCandidate,
};

thread_local! {
    static DETECTOR: Retained<NSDataDetector> = NSDataDetector::dataDetectorWithTypes_error(
        (NSTextCheckingType::Link | NSTextCheckingType::PhoneNumber).bits(),
    ).expect("Link and phone checking types are valid");
}

pub(super) fn tokens(text: &str) -> Vec<OcrCopyToken> {
    let text = normalize_ocr_text(text, None);
    let mut byte_offsets = Vec::with_capacity(text.len() + 1);
    for (byte, character) in text.char_indices() {
        byte_offsets.extend(std::iter::repeat_n(byte, character.len_utf16()));
    }
    byte_offsets.push(text.len());
    let candidates = DETECTOR.with(|detector| {
        let string = NSString::from_str(&text);
        detector
            .matchesInString_options_range(
                &string,
                NSMatchingOptions::empty(),
                NSRange::new(0, string.length()),
            )
            .iter()
            .filter_map(|matched| {
                let range = matched.range();
                let start = *byte_offsets.get(range.location)?;
                let end = *byte_offsets.get(range.location.checked_add(range.length)?)?;
                let value = text.get(start..end)?.to_owned();
                let (label, priority) = if matched.resultType() == NSTextCheckingType::PhoneNumber {
                    if value.parse::<std::net::IpAddr>().is_ok() {
                        return None;
                    }
                    ("电话", 94)
                } else if matched.URL().is_some_and(|url| {
                    url.scheme()
                        .is_some_and(|scheme| scheme.to_string() == "mailto")
                }) {
                    ("邮箱", 100)
                } else {
                    ("网址", 98)
                };
                Some(TokenCandidate {
                    label: label.into(),
                    value,
                    start,
                    end,
                    priority,
                })
            })
            .collect()
    });
    copy_tokens(&text, candidates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_detector_preserves_contact_values_unicode_offsets_and_wrapped_tokens() {
        let values = tokens("📷 官网 https://example.com/docs?a=1, 邮箱 support@example.com\n电话 138-0013-8000 或 +1 (650) 253-0000\n内网 192.168.1.8:8080，订单号 AB-1234567890，验证码 123456");
        assert_eq!(
            values
                .iter()
                .map(|token| (token.label.as_str(), token.value.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("网址", "https://example.com/docs?a=1"),
                ("邮箱", "support@example.com"),
                ("电话", "138-0013-8000"),
                ("电话", "+1 (650) 253-0000"),
                ("地址", "192.168.1.8:8080"),
                ("编号", "AB-1234567890"),
                ("验证码", "123456"),
            ]
        );
        assert_eq!(
            tokens("网址 https://example.\ncom/docs?a=1\n电话 138-0013-\n8000")
                .iter()
                .map(|token| token.value.as_str())
                .collect::<Vec<_>>(),
            vec!["https://example.com/docs?a=1", "138-0013-8000"]
        );
    }

    #[test]
    fn native_tokens_keep_daily_copy_targets_without_treating_ids_as_phone_numbers() {
        let values = tokens("验证码 8K2P91，金额 ¥128.00，预约时间 2026-07-05 14:30\nWi-Fi: Cafe_5G 密码: snap-2026\n车牌 沪A12345 航班 MU5137 车次 G1234 座位 12A\n账号 @snaplingo 坐标 31.2304, 121.4737 身份证 110105199003071234");
        assert_eq!(
            values
                .iter()
                .map(|token| (token.label.as_str(), token.value.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("验证码", "8K2P91"),
                ("金额", "¥128.00"),
                ("时间", "2026-07-05 14:30"),
                ("账号", "Cafe_5G"),
                ("密码", "snap-2026"),
                ("车牌", "沪A12345"),
                ("航班", "MU5137"),
                ("车次", "G1234"),
                ("座位", "12A"),
                ("账号", "@snaplingo"),
                ("坐标", "31.2304, 121.4737"),
                ("敏感信息", "110105199003071234"),
            ]
        );
        assert_eq!(
            tokens("访问 example.com，联系电话 138 0013 8000，错误短号 12345")
                .iter()
                .map(|token| token.value.as_str())
                .collect::<Vec<_>>(),
            vec!["example.com", "138 0013 8000"]
        );
    }
}
