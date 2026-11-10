use super::*;

pub fn language_name(code: &str) -> &str {
    let code = if code == "zh" { "zh-CN" } else { code };
    LANGUAGES
        .iter()
        .find(|(id, _)| *id == code)
        .and_then(|(_, label)| label.split_whitespace().next())
        .unwrap_or(code)
}

/// Mac's density thresholds are independent of the platform's text measurement.
pub fn text_size(text: &str, source: bool) -> f32 {
    let lines = text
        .lines()
        .map(|line| {
            (line.chars().count() as f32 / if source { 54. } else { 72. })
                .ceil()
                .max(1.)
        })
        .sum::<f32>()
        .max(1.);
    let density = (text.chars().count() as f32 / if source { 140. } else { 180. })
        .max(lines / if source { 2. } else { 3. });
    if density >= 2.2 {
        12.
    } else if density >= 1.15 {
        13.
    } else {
        14.
    }
}

#[derive(Clone, Debug)]
pub struct ResultButton {
    pub target: ResultTarget,
    pub label: &'static str,
    pub enabled: bool,
}

fn button(target: ResultTarget, label: &'static str, enabled: bool) -> ResultButton {
    ResultButton {
        target,
        label,
        enabled,
    }
}

pub struct ResultRowPresentation<'a> {
    pub status: &'static str,
    pub text: &'a str,
    pub error: bool,
    pub buttons: [ResultButton; 4],
}

impl ResultRow {
    pub fn presentation(&self, favorite_pending: bool) -> ResultRowPresentation<'_> {
        let pending = self.status == RowStatus::Pending;
        let error = self.error.is_some();
        let available = !pending && !error && !self.text.is_empty();
        ResultRowPresentation {
            status: if pending {
                "翻译中"
            } else if error {
                "失败"
            } else {
                ""
            },
            text: if pending {
                "正在请求翻译服务…"
            } else {
                self.error.as_deref().unwrap_or(&self.text)
            },
            error,
            buttons: [
                button(
                    ResultTarget::Retry(self.id.clone()),
                    "重试",
                    !pending && (!error || self.retryable),
                ),
                button(
                    ResultTarget::SpeakResult(self.id.clone()),
                    "朗读",
                    available,
                ),
                button(
                    ResultTarget::FavoriteResult(self.id.clone()),
                    if self.favorite { "已收藏" } else { "收藏" },
                    available && !self.favorite && !favorite_pending,
                ),
                button(ResultTarget::CopyResult(self.id.clone()), "复制", available),
            ],
        }
    }
}

pub struct ResultPresentation {
    pub title: &'static str,
    pub subtitle: String,
    pub source_metadata: String,
    pub collapsed_source: String,
    pub source_buttons: [ResultButton; 6],
    pub image_title: &'static str,
    pub ocr_title: &'static str,
    pub ocr_metadata: String,
    pub ocr_language: String,
    pub upload: ResultButton,
    pub ocr_buttons: [ResultButton; 4],
}

impl ResultSnapshot {
    pub(super) fn enabled(&self, target: &ResultTarget) -> bool {
        let translation = self.mode == ResultMode::Translation;
        match target {
            ResultTarget::Close | ResultTarget::Pin | ResultTarget::Header => true,
            ResultTarget::Source => translation,
            ResultTarget::OcrText => !translation,
            ResultTarget::SourceLanguage | ResultTarget::TargetLanguage | ResultTarget::Swap => {
                translation
            }
            ResultTarget::Language(_, code) => {
                translation && LANGUAGES.iter().any(|(id, _)| id == code)
            }
            ResultTarget::ToggleRow(id) | ResultTarget::ResultText(id) => {
                translation && self.rows.iter().any(|row| row.id == *id)
            }
            ResultTarget::CopyFragments => !translation && !self.ocr_tokens.is_empty(),
            ResultTarget::CopyToken(value) => {
                !translation && self.ocr_tokens.iter().any(|token| token.value == *value)
            }
            _ => {
                let presentation = self.presentation();
                if translation {
                    presentation
                        .source_buttons
                        .into_iter()
                        .chain(
                            self.rows
                                .iter()
                                .flat_map(|row| row.presentation(self.favorite_pending).buttons),
                        )
                        .any(|button| button.target == *target && button.enabled)
                } else {
                    presentation
                        .ocr_buttons
                        .iter()
                        .chain(std::iter::once(&presentation.upload))
                        .any(|button| button.target == *target && button.enabled)
                }
            }
        }
    }

    pub fn has_ocr_image(&self) -> bool {
        self.ocr_image_present || self.ocr_image.is_some()
    }

    pub fn presentation(&self) -> ResultPresentation {
        let translation = self.mode == ResultMode::Translation;
        let source = !self.source.trim().is_empty();
        let ocr = !self.ocr_text.trim().is_empty();
        let completed: Vec<_> = self
            .rows
            .iter()
            .filter(|row| row.status == RowStatus::Success && !row.text.is_empty())
            .collect();
        let all_favorite = !completed.is_empty() && completed.iter().all(|row| row.favorite);
        let subtitle = if translation {
            if self.rows.is_empty() {
                "准备翻译".into()
            } else {
                format!(
                    "{}/{} 个服务已返回",
                    self.rows
                        .iter()
                        .filter(|row| row.status == RowStatus::Success)
                        .count(),
                    self.rows.len()
                )
            }
        } else if self.ocr_running {
            "识别中".into()
        } else if self.ocr_text.is_empty() {
            "等待上传".into()
        } else {
            let mut text = format!("已识别 {} 个字符", self.ocr_text.encode_utf16().count());
            if let Some(confidence) = self.ocr_confidence {
                text.push_str(&format!(" · 置信度 {:.0}%", confidence * 100.));
            }
            text
        };
        ResultPresentation {
            title: if translation { "翻译" } else { "OCR" },
            subtitle,
            source_metadata: format!(
                "{} chars · 识别为 {}",
                self.source.encode_utf16().count(),
                language_name(
                    self.detected_language
                        .as_deref()
                        .unwrap_or(&self.source_language)
                )
            ),
            collapsed_source: format!(
                "{}    · 展开原文",
                self.source
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .chars()
                    .take(80)
                    .collect::<String>()
            ),
            source_buttons: [
                button(ResultTarget::ToggleSource, "折叠", true),
                button(
                    ResultTarget::FavoriteAll,
                    if all_favorite {
                        "已收藏"
                    } else {
                        "收藏全部结果"
                    },
                    !completed.is_empty() && !all_favorite && !self.favorite_pending,
                ),
                button(ResultTarget::CopySource, "复制原文", source),
                button(ResultTarget::SpeakSource, "朗读原文", source),
                button(
                    ResultTarget::Translate,
                    "翻译 / 重试",
                    source && !self.translating && !self.composing,
                ),
                button(
                    ResultTarget::ClearSource,
                    "清空原文",
                    !self.source.is_empty(),
                ),
            ],
            image_title: if self.has_ocr_image() {
                "原图"
            } else {
                "图片 · PNG/JPG/WebP"
            },
            ocr_title: if self.has_ocr_image() {
                "OCR 结果"
            } else {
                "识别文本"
            },
            ocr_metadata: format!("{} chars", self.ocr_text.encode_utf16().count()),
            ocr_language: format!("识别语言：{}", self.ocr_language.as_deref().unwrap_or("")),
            upload: button(
                ResultTarget::FileOcr,
                if self.ocr_running {
                    "识别中…"
                } else {
                    "上传图片 OCR"
                },
                !self.ocr_running && !self.dialog_open,
            ),
            ocr_buttons: [
                button(ResultTarget::SpeakOcr, "朗读", ocr),
                button(
                    ResultTarget::FavoriteOcr,
                    if self.ocr_favorite {
                        "已收藏"
                    } else {
                        "收藏"
                    },
                    ocr && !self.ocr_favorite && !self.favorite_pending,
                ),
                button(ResultTarget::CopyOcr, "复制", ocr),
                button(ResultTarget::ClearOcr, "清空", !self.ocr_text.is_empty()),
            ],
        }
    }
}
