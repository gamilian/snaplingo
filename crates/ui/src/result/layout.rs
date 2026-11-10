use super::*;
use crate::Size;

fn r(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect {
        x,
        y,
        width: width.max(0.),
        height: height.max(0.),
    }
}

/// TextKit and the CPU renderer supply their own measured text heights.
#[derive(Default)]
pub struct ResultMeasurements {
    pub source: f32,
    pub ocr: f32,
    pub error: f32,
    pub rows: Vec<f32>,
    pub image: Option<Size>,
}

pub struct ResultSourceLayout {
    pub frame: Rect,
    pub text: Rect,
    pub placeholder: Rect,
    pub metadata: Rect,
    pub buttons: [Rect; 6],
}
pub struct ResultRowLayout {
    pub frame: Rect,
    pub title: Rect,
    pub status: Rect,
    pub text: Rect,
    pub buttons: [Rect; 4],
}
pub struct ResultOcrLayout {
    pub language: Option<Rect>,
    pub image_title: Rect,
    pub image: Option<Rect>,
    pub upload: Rect,
    pub title: Rect,
    pub frame: Rect,
    pub text: Rect,
    pub placeholder: Rect,
    pub fragments: Rect,
    pub metadata: Rect,
    pub buttons: [Rect; 4],
}
pub struct ResultLayout {
    pub source: Option<ResultSourceLayout>,
    pub collapsed_source: Option<Rect>,
    pub languages: Option<[Rect; 3]>,
    pub error: Option<Rect>,
    pub rows: Vec<ResultRowLayout>,
    pub ocr: Option<ResultOcrLayout>,
    pub content_height: f32,
}
pub struct ResultHeaderLayout {
    pub pin: Rect,
    pub title: Rect,
    pub subtitle: Rect,
    pub close: Rect,
    pub viewport: Rect,
}

impl ResultLayout {
    pub fn header(size: Size) -> ResultHeaderLayout {
        ResultHeaderLayout {
            pin: r(10., 10., 28., 28.),
            title: r(48., 12., 52., 24.),
            subtitle: r(106., 16., size.width - 150., 20.),
            close: r(size.width - 38., 10., 28., 28.),
            viewport: r(12., 60., size.width - 24., size.height - 72.),
        }
    }

    pub fn window_height(&self, maximum: f32) -> f32 {
        (self.content_height + 72.).min(maximum).max(140.)
    }
}

impl ResultUi {
    pub fn source_collapsed(&self) -> bool {
        self.source_collapsed
    }

    /// Platform persistence decides which remembered value to restore.
    pub fn restore_source_collapsed(&mut self, collapsed: bool) {
        self.source_collapsed = collapsed;
    }

    /// All rectangles are logical points. Card contents are relative to their card;
    /// other body rectangles are relative to the scrolling document.
    pub fn layout(&self, width: f32, measured: &ResultMeasurements) -> ResultLayout {
        let width = (width - 24.).max(1.);
        let narrow = width < 430.;
        let mut layout = ResultLayout {
            source: None,
            collapsed_source: None,
            languages: None,
            error: None,
            rows: vec![],
            ocr: None,
            content_height: 0.,
        };
        let mut y = 0.;
        let translation = self.snapshot.mode == ResultMode::Translation;
        if translation {
            if self.source_collapsed {
                layout.collapsed_source = Some(r(0., y, width, 40.));
                y += 52.;
            } else {
                let body = measured.source.max(52.);
                let footer = if narrow { 62. } else { 38. };
                let mut x = width - 206.;
                layout.source = Some(ResultSourceLayout {
                    frame: r(0., y, width, body + footer + 2.),
                    text: r(2., 2., width - 4., body),
                    placeholder: r(12., 10., width - 24., 24.),
                    metadata: r(
                        12.,
                        body + 10.,
                        width - if narrow { 24. } else { 226. },
                        20.,
                    ),
                    buttons: std::array::from_fn(|index| {
                        let size = if index == 0 { 56. } else { 28. };
                        let rect = r(x, body + if narrow { 30. } else { 4. }, size, 30.);
                        x += size + 1.;
                        rect
                    }),
                });
                y += body + footer + 14.;
            }
            let half = (width - 40.) / 2.;
            layout.languages = Some([
                r(0., y, half, 36.),
                r(half, y, 40., 36.),
                r(half + 40., y, half, 36.),
            ]);
            y += 48.;
        } else {
            let language = self.snapshot.ocr_language.as_ref().map(|_| {
                let rect = r(0., y, width, 20.);
                y += 26.;
                rect
            });
            let image_title = r(0., y, width, 20.);
            y += 26.;
            let image = measured.image.map(|size| {
                let height = (size.height * width / size.width.max(1.)).clamp(48., 220.);
                let rect = r(0., y, width, height);
                y += height + 8.;
                rect
            });
            let upload = r(0., y, width, if image.is_some() { 34. } else { 84. });
            y += upload.height + 12.;
            layout.ocr = Some(ResultOcrLayout {
                language,
                image_title,
                image,
                upload,
                title: Rect::default(),
                frame: Rect::default(),
                text: Rect::default(),
                placeholder: Rect::default(),
                fragments: Rect::default(),
                metadata: Rect::default(),
                buttons: [Rect::default(); 4],
            });
        }
        if self.snapshot.error.is_some() {
            let height = measured.error.max(40.);
            layout.error = Some(r(0., y, width, height));
            y += height + 12.;
        }
        if translation {
            for (index, row) in self.snapshot.rows.iter().enumerate() {
                let header = if narrow { 62. } else { 36. };
                let body = if self.row_expanded(&row.id) {
                    measured.rows.get(index).copied().unwrap_or(0.).max(44.)
                } else {
                    0.
                };
                let frame = r(0., y, width, header + body + 2.);
                layout.rows.push(ResultRowLayout {
                    frame,
                    title: r(
                        8.,
                        4.,
                        (width - if narrow { 70. } else { 180. }).max(40.),
                        28.,
                    ),
                    status: r(width - if narrow { 62. } else { 170. }, 10., 48., 18.),
                    text: r(2., header, width - 4., body),
                    buttons: std::array::from_fn(|index| {
                        r(
                            width - 116. + index as f32 * 26.,
                            if narrow { 32. } else { 4. },
                            24.,
                            28.,
                        )
                    }),
                });
                y += frame.height + 12.;
            }
        } else if let Some(ocr) = &mut layout.ocr {
            ocr.title = r(0., y, width, 20.);
            y += 26.;
            let body = measured.ocr.max(72.);
            let tokens = if self.snapshot.ocr_tokens.is_empty() {
                0.
            } else {
                34.
            };
            ocr.frame = r(0., y, width, body + tokens + 40.);
            ocr.text = r(2., 2., width - 4., body);
            ocr.placeholder = r(12., 10., width - 24., 24.);
            ocr.fragments = r(8., body + 2., width - 16., tokens);
            ocr.metadata = r(12., body + tokens + 10., width - 136., 20.);
            ocr.buttons = std::array::from_fn(|index| {
                r(
                    width - 120. + index as f32 * 28.,
                    body + tokens + 4.,
                    28.,
                    30.,
                )
            });
            y += body + tokens + 52.;
        }
        layout.content_height = y;
        layout
    }
}
