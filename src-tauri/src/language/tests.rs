#[cfg(test)]
mod tests {
    use super::super::detector::LanguageDetector;

    #[test]
    fn test_detect_chinese() {
        let detector = LanguageDetector::new();
        let lang = detector.detect("你好世界").unwrap();
        assert_eq!(lang, "zh-CN");
    }

    #[test]
    fn test_detect_english() {
        let detector = LanguageDetector::new();
        let lang = detector.detect("Hello world").unwrap();
        assert_eq!(lang, "en");
    }

    #[test]
    fn test_smart_target_chinese_to_english() {
        let detector = LanguageDetector::new();
        assert_eq!(detector.smart_target("zh"), "en");
        assert_eq!(detector.smart_target("zh-CN"), "en");
    }

    #[test]
    fn test_smart_target_other_to_chinese() {
        let detector = LanguageDetector::new();
        assert_eq!(detector.smart_target("en"), "zh-CN");
        assert_eq!(detector.smart_target("ja"), "zh-CN");
        assert_eq!(detector.smart_target("es"), "zh-CN");
    }
}
