use crate::domain::capture::{CaptureCandidateView, LogicalPoint, LogicalRect};

const MIN_SIZE: f64 = 2.0;
const SNAP_DISTANCE: f64 = 6.0;
const HISTORY_LIMIT: usize = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CaptureSelectionMode {
    Edit,
    Copy,
    Ocr,
    SilentOcr,
    Translate,
}

impl CaptureSelectionMode {
    pub fn parse(mode: &str) -> Option<Self> {
        match mode {
            "screenshot" => Some(Self::Edit),
            "screenshot-copy" => Some(Self::Copy),
            "screenshot-ocr" => Some(Self::Ocr),
            "silent-screenshot-ocr" => Some(Self::SilentOcr),
            "screenshot-translate" => Some(Self::Translate),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Edit => "screenshot",
            Self::Copy => "screenshot-copy",
            Self::Ocr => "screenshot-ocr",
            Self::SilentOcr => "silent-screenshot-ocr",
            Self::Translate => "screenshot-translate",
        }
    }
}

/// Desktop coordinates stay independent of the native window's local origin.
pub(crate) struct Selection {
    pub bounds: LogicalRect,
    pub point: Option<LogicalPoint>,
    pub rect: Option<LogicalRect>,
    pub hover: Option<LogicalRect>,
    pub anchor: Option<LogicalPoint>,
    pub control_detection: bool,
    pub revision: u64,
    candidates: Vec<CaptureCandidateView>,
    snap_targets: Vec<LogicalRect>,
    pressed_hover: Option<LogicalRect>,
    keyboard_adjusted: bool,
}

impl Selection {
    pub fn new(
        bounds: LogicalRect,
        monitors: Vec<LogicalRect>,
        candidates: Vec<CaptureCandidateView>,
    ) -> Self {
        let snap_targets = monitors
            .iter()
            .cloned()
            .chain(candidates.iter().map(|candidate| candidate.rect.clone()))
            .collect();
        let mut candidates = candidates
            .into_iter()
            .filter(|candidate| {
                candidate.kind != "monitor"
                    && candidate.rect.width >= 48.0
                    && candidate.rect.height >= 48.0
                    && !(candidate.kind == "window"
                        && monitors
                            .iter()
                            .any(|monitor| nearly_equal(&candidate.rect, monitor)))
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|a, b| {
            b.priority.cmp(&a.priority).then_with(|| {
                (a.rect.width * a.rect.height).total_cmp(&(b.rect.width * b.rect.height))
            })
        });
        Self {
            bounds,
            point: None,
            rect: None,
            hover: None,
            anchor: None,
            control_detection: false,
            revision: 0,
            candidates,
            snap_targets,
            pressed_hover: None,
            keyboard_adjusted: false,
        }
    }

    pub fn move_to(&mut self, point: LogicalPoint, square: bool) -> bool {
        if self.point.as_ref() == Some(&point) && self.anchor.is_none() {
            return false;
        }
        self.revision = self.revision.wrapping_add(1);
        self.keyboard_adjusted = false;
        if let Some(anchor) = &self.anchor {
            self.rect = Some(self.draft_rect(anchor, &point, square));
        } else {
            self.hover = if self.control_detection {
                self.hover.take().filter(|rect| contains(rect, &point))
            } else {
                self.candidate_at(&point)
            };
        }
        self.point = Some(point);
        true
    }

    pub fn press(&mut self, point: LogicalPoint, retain_hover: bool) {
        self.move_to(point.clone(), false);
        self.pressed_hover = self
            .hover
            .clone()
            .filter(|rect| retain_hover && contains(rect, &point));
        self.anchor = Some(self.snap(&point));
        self.rect = None;
        self.hover = None;
        self.keyboard_adjusted = false;
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn release(&mut self, point: LogicalPoint, square: bool) -> Option<LogicalRect> {
        let anchor = self.anchor.take()?;
        let point = if self.keyboard_adjusted {
            self.point.clone().unwrap_or(point)
        } else {
            point
        };
        let draft = self.draft_rect(&anchor, &point, square);
        let rect = if valid_size(&draft) {
            Some(draft)
        } else {
            self.pressed_hover
                .take()
                .filter(|rect| contains(rect, &point))
                .or_else(|| self.candidate_at(&point))
        };
        self.rect = rect.clone();
        self.hover = None;
        self.pressed_hover = None;
        self.point = Some(point);
        self.keyboard_adjusted = false;
        self.revision = self.revision.wrapping_add(1);
        rect
    }

    pub fn reset(&mut self) {
        self.anchor = None;
        self.rect = None;
        self.hover = None;
        self.pressed_hover = None;
        self.keyboard_adjusted = false;
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn toggle_detection(&mut self) {
        self.control_detection = !self.control_detection;
        self.revision = self.revision.wrapping_add(1);
        self.hover = if self.control_detection || self.anchor.is_some() {
            None
        } else {
            self.point
                .as_ref()
                .and_then(|point| self.candidate_at(point))
        };
    }

    pub fn control_result(&mut self, revision: u64, rect: Option<LogicalRect>) -> bool {
        if revision != self.revision || !self.control_detection || self.anchor.is_some() {
            return false;
        }
        self.hover = rect;
        true
    }

    pub fn control_failed(&mut self, revision: u64) -> bool {
        if revision != self.revision || !self.control_detection || self.anchor.is_some() {
            return false;
        }
        self.toggle_detection();
        true
    }

    pub fn nudge_cursor(&mut self, dx: f64, dy: f64) -> Option<LogicalPoint> {
        let old = self.point.clone()?;
        let next = self.clamp_point(LogicalPoint {
            x: old.x + dx,
            y: old.y + dy,
        });
        let delta = LogicalPoint {
            x: next.x - old.x,
            y: next.y - old.y,
        };
        self.move_to(next.clone(), false);
        if let Some(anchor) = &self.anchor {
            self.rect = Some(normalize(anchor, &next));
            self.keyboard_adjusted = true;
        }
        Some(delta)
    }

    pub fn active_rect(&self) -> Option<&LogicalRect> {
        self.rect.as_ref().or(self.hover.as_ref())
    }

    fn candidate_at(&self, point: &LogicalPoint) -> Option<LogicalRect> {
        self.candidates
            .iter()
            .find(|candidate| contains(&candidate.rect, point))
            .map(|candidate| candidate.rect.clone())
    }

    fn clamp_point(&self, point: LogicalPoint) -> LogicalPoint {
        LogicalPoint {
            x: point
                .x
                .clamp(self.bounds.x, self.bounds.x + self.bounds.width - 1.0),
            y: point
                .y
                .clamp(self.bounds.y, self.bounds.y + self.bounds.height - 1.0),
        }
    }

    fn snap(&self, point: &LogicalPoint) -> LogicalPoint {
        let nearest = |value: f64, horizontal: bool| {
            let mut distance = SNAP_DISTANCE;
            let mut snapped = value;
            for target in &self.snap_targets {
                let (start, extent) = if horizontal {
                    (target.x, target.width)
                } else {
                    (target.y, target.height)
                };
                for edge in [start, start + extent] {
                    if (edge - value).abs() <= distance {
                        distance = (edge - value).abs();
                        snapped = edge;
                    }
                }
            }
            snapped
        };
        LogicalPoint {
            x: nearest(point.x, true),
            y: nearest(point.y, false),
        }
    }

    fn draft_rect(&self, anchor: &LogicalPoint, point: &LogicalPoint, square: bool) -> LogicalRect {
        let mut end = self.snap(point);
        if square {
            let dx = end.x - anchor.x;
            let dy = end.y - anchor.y;
            let size = dx.abs().min(dy.abs());
            end = LogicalPoint {
                x: anchor.x + dx.signum() * size,
                y: anchor.y + dy.signum() * size,
            };
        }
        normalize(anchor, &end)
    }
}

fn normalize(a: &LogicalPoint, b: &LogicalPoint) -> LogicalRect {
    LogicalRect {
        x: a.x.min(b.x),
        y: a.y.min(b.y),
        width: (a.x - b.x).abs(),
        height: (a.y - b.y).abs(),
    }
}

pub(crate) fn contains(rect: &LogicalRect, point: &LogicalPoint) -> bool {
    point.x >= rect.x
        && point.x < rect.x + rect.width
        && point.y >= rect.y
        && point.y < rect.y + rect.height
}

fn valid_size(rect: &LogicalRect) -> bool {
    rect.x.is_finite()
        && rect.y.is_finite()
        && rect.width.is_finite()
        && rect.height.is_finite()
        && rect.width >= MIN_SIZE
        && rect.height >= MIN_SIZE
}

fn nearly_equal(a: &LogicalRect, b: &LogicalRect) -> bool {
    (a.x - b.x).abs() <= 1.0
        && (a.y - b.y).abs() <= 1.0
        && (a.width - b.width).abs() <= 1.0
        && (a.height - b.height).abs() <= 1.0
}

pub(crate) fn restore_selection(rect: &LogicalRect, bounds: &LogicalRect) -> Option<LogicalRect> {
    if !valid_size(rect) || rect.width > bounds.width || rect.height > bounds.height {
        return None;
    }
    Some(LogicalRect {
        x: rect.x.clamp(bounds.x, bounds.x + bounds.width - rect.width),
        y: rect
            .y
            .clamp(bounds.y, bounds.y + bounds.height - rect.height),
        ..rect.clone()
    })
}

pub(crate) fn merge_selection_history(
    current: Vec<LogicalRect>,
    imported: Vec<LogicalRect>,
) -> Vec<LogicalRect> {
    let mut history = Vec::new();
    for rect in current.into_iter().chain(imported).filter(valid_size) {
        if !history.contains(&rect) {
            history.push(rect);
            if history.len() == HISTORY_LIMIT {
                break;
            }
        }
    }
    history
}

pub(crate) fn remember_selection(history: Vec<LogicalRect>, rect: LogicalRect) -> Vec<LogicalRect> {
    merge_selection_history(vec![rect], history)
}

#[cfg(test)]
#[path = "selection_test.rs"]
mod tests;
