use std::collections::{BTreeSet, HashMap, VecDeque};

use image::RgbaImage;

#[cfg(test)]
const MAX_HEIGHT: usize = 30_000;
const MAX_PIXELS: usize = 50_000_000;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ScrollingStep {
    Appended(usize),
    Still,
    Failed,
    Limit,
}

struct Segment {
    prepend: bool,
    rows: usize,
    previous_offset: i64,
}

pub(super) struct ScrollingImage {
    width: u32,
    frame_height: u32,
    rows: VecDeque<Box<[u8]>>,
    header: Vec<Box<[u8]>>,
    footer: Vec<Box<[u8]>>,
    previous: RgbaImage,
    offset: i64,
    initialized: bool,
    history: Vec<Segment>,
    ignored_frame: Option<Vec<u64>>,
    max_height: usize,
}

impl ScrollingImage {
    pub fn validate_dimensions_with_limit(
        width: u32,
        height: u32,
        max_height: u32,
    ) -> crate::Result<()> {
        if width == 0
            || height == 0
            || height > max_height
            || u64::from(width) * u64::from(height) > MAX_PIXELS as u64
        {
            return Err("Scrolling capture viewport is too large or empty".into());
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn new(frame: RgbaImage) -> crate::Result<Self> {
        Self::with_max_height(frame, MAX_HEIGHT as u32)
    }

    pub fn with_max_height(frame: RgbaImage, max_height: u32) -> crate::Result<Self> {
        let (width, height) = frame.dimensions();
        Self::validate_dimensions_with_limit(width, height, max_height)?;
        Ok(Self {
            width,
            frame_height: height,
            rows: frame_rows(&frame, 0, height as usize).into(),
            header: Vec::new(),
            footer: Vec::new(),
            previous: frame,
            offset: 0,
            initialized: false,
            history: Vec::new(),
            ignored_frame: None,
            max_height: (max_height as usize).min(MAX_PIXELS / width as usize),
        })
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (
            self.width,
            (self.header.len() + self.rows.len() + self.footer.len()) as u32,
        )
    }
    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    pub fn at_limit(&self) -> bool {
        self.dimensions().1 as usize >= self.max_height
    }

    pub fn push(&mut self, frame: RgbaImage) -> ScrollingStep {
        if self.at_limit() {
            return ScrollingStep::Limit;
        }
        if frame.dimensions() != (self.width, self.frame_height) {
            return ScrollingStep::Failed;
        }
        let next_hashes = row_hashes(&frame);
        let body_start = self.header.len();
        let body_end = next_hashes.len() - self.footer.len();
        if self.ignored_frame.as_deref() == Some(&next_hashes[body_start..body_end]) {
            return ScrollingStep::Still;
        }
        self.ignored_frame = None;
        if frame == self.previous {
            return ScrollingStep::Still;
        }
        let previous_hashes = row_hashes(&self.previous);
        if previous_hashes[body_start..body_end] == next_hashes[body_start..body_end] {
            return ScrollingStep::Still;
        }
        let (top, bottom) = if self.initialized {
            (self.header.len(), self.footer.len())
        } else {
            fixed_regions(&self.previous, &previous_hashes, &next_hashes)
        };
        let Some(delta) = locate(
            &self.previous,
            &frame,
            &previous_hashes,
            &next_hashes,
            top,
            bottom,
        ) else {
            return if changed_fraction(&self.previous, &frame) < 0.002 {
                ScrollingStep::Still
            } else {
                ScrollingStep::Failed
            };
        };
        if !self.initialized {
            self.header = frame_rows(&self.previous, 0, top);
            self.footer = frame_rows(
                &self.previous,
                self.frame_height as usize - bottom,
                self.frame_height as usize,
            );
            self.rows = frame_rows(&self.previous, top, self.frame_height as usize - bottom).into();
            self.initialized = true;
        }
        let body_height = self.frame_height as usize - top - bottom;
        let next_offset = self.offset + delta;
        let prepend = next_offset < 0;
        let added = if prepend {
            (-next_offset) as usize
        } else {
            (next_offset + body_height as i64 - self.rows.len() as i64).max(0) as usize
        };
        let requested = added;
        let added = added.min(self.max_height - self.dimensions().1 as usize);
        if added > 0 {
            let new_rows = if prepend {
                frame_rows(&frame, top + requested - added, top + requested)
            } else {
                frame_rows(
                    &frame,
                    self.frame_height as usize - bottom - requested,
                    self.frame_height as usize - bottom - requested + added,
                )
            };
            self.history.push(Segment {
                prepend,
                rows: added,
                previous_offset: self.offset,
            });
            if prepend {
                for row in new_rows.into_iter().rev() {
                    self.rows.push_front(row);
                }
            } else {
                self.rows.extend(new_rows);
            }
        }
        self.offset = next_offset
            .max(0)
            .min((self.rows.len() - body_height) as i64);
        self.previous = frame;
        if self.at_limit() {
            ScrollingStep::Limit
        } else if added > 0 {
            ScrollingStep::Appended(added)
        } else {
            ScrollingStep::Still
        }
    }

    pub fn undo(&mut self) -> bool {
        let Some(segment) = self.history.pop() else {
            return false;
        };
        // A stationary page must not immediately re-add the undone segment.
        let hashes = row_hashes(&self.previous);
        self.ignored_frame =
            Some(hashes[self.header.len()..hashes.len() - self.footer.len()].to_vec());
        for _ in 0..segment.rows {
            if segment.prepend {
                self.rows.pop_front();
            } else {
                self.rows.pop_back();
            }
        }
        self.offset = segment.previous_offset;
        let body_height = self.frame_height as usize - self.header.len() - self.footer.len();
        let mut pixels = Vec::with_capacity(self.width as usize * self.frame_height as usize * 4);
        for row in &self.header {
            pixels.extend_from_slice(row);
        }
        for row in self
            .rows
            .iter()
            .skip(self.offset as usize)
            .take(body_height)
        {
            pixels.extend_from_slice(row);
        }
        for row in &self.footer {
            pixels.extend_from_slice(row);
        }
        self.previous = RgbaImage::from_raw(self.width, self.frame_height, pixels)
            .expect("viewport rows retain their size");
        true
    }

    fn row(&self, index: usize) -> &[u8] {
        if index < self.header.len() {
            &self.header[index]
        } else if index < self.header.len() + self.rows.len() {
            &self.rows[index - self.header.len()]
        } else {
            &self.footer[index - self.header.len() - self.rows.len()]
        }
    }

    pub fn image(&self) -> RgbaImage {
        let (width, height) = self.dimensions();
        let mut bytes = Vec::with_capacity(width as usize * height as usize * 4);
        for y in 0..height as usize {
            bytes.extend_from_slice(self.row(y));
        }
        RgbaImage::from_raw(width, height, bytes).expect("scrolling image rows retain their size")
    }

    pub fn thumbnail(&self, max_width: u32, max_height: u32) -> RgbaImage {
        let (width, height) = self.dimensions();
        let scale = (max_width as f64 / width as f64)
            .min(max_height as f64 / height as f64)
            .min(1.0);
        let target_width = (width as f64 * scale).round().max(1.0) as u32;
        let target_height = (height as f64 * scale).round().max(1.0) as u32;
        RgbaImage::from_fn(target_width, target_height, |x, y| {
            let source_x = (x as u64 * width as u64 / target_width as u64) as usize;
            let source_y = (y as u64 * height as u64 / target_height as u64) as usize;
            let row = self.row(source_y);
            image::Rgba(row[source_x * 4..source_x * 4 + 4].try_into().unwrap())
        })
    }
}

#[cfg(test)]
fn within_limits(width: u32, height: usize) -> bool {
    width > 0 && height > 0 && height <= MAX_HEIGHT && width as usize <= MAX_PIXELS / height
}

fn matching_columns(frame: &RgbaImage) -> std::ops::Range<u32> {
    // Scrollbars and narrow window borders must not determine row alignment.
    let inset = (frame.width() / 10).min(24);
    inset..frame.width() - inset
}

fn frame_rows(frame: &RgbaImage, start: usize, end: usize) -> Vec<Box<[u8]>> {
    let stride = frame.width() as usize * 4;
    frame.as_raw()[start * stride..end * stride]
        .chunks_exact(stride)
        .map(|row| row.to_vec().into_boxed_slice())
        .collect()
}

fn row_hashes(frame: &RgbaImage) -> Vec<u64> {
    (0..frame.height())
        .map(|y| {
            let mut hash = 0xcbf29ce484222325u64;
            for x in matching_columns(frame).step_by(4) {
                for channel in frame.get_pixel(x, y).0 {
                    hash = (hash ^ (channel >> 3) as u64).wrapping_mul(0x100000001b3);
                }
            }
            hash
        })
        .collect()
}

fn detailed_region(frame: &RgbaImage, start: usize, end: usize, adjacent: usize) -> bool {
    if start == end {
        return false;
    }
    let left = matching_columns(frame).start;
    let base = frame.get_pixel(left, start as u32);
    let next = frame.get_pixel(left, adjacent.min(frame.height() as usize - 1) as u32);
    if (0..3).any(|c| base[c].abs_diff(next[c]) > 12) {
        return true;
    }
    (start..end).any(|y| {
        matching_columns(frame)
            .step_by(4)
            .any(|x| frame.get_pixel(x, y as u32) != frame.get_pixel(left, y as u32))
    })
}

fn fixed_regions(previous: &RgbaImage, old: &[u64], new: &[u64]) -> (usize, usize) {
    let height = old.len();
    let mut top = old
        .iter()
        .zip(new)
        .take_while(|(a, b)| a == b)
        .count()
        .min(height / 3);
    let mut bottom = old
        .iter()
        .rev()
        .zip(new.iter().rev())
        .take_while(|(a, b)| a == b)
        .count()
        .min(height / 3);
    if top < 8 || !detailed_region(previous, 0, top, top) {
        top = 0;
    }
    if bottom < 8
        || !detailed_region(
            previous,
            height - bottom,
            height,
            height.saturating_sub(bottom + 1),
        )
    {
        bottom = 0;
    }
    (top, bottom)
}

fn locate(
    previous: &RgbaImage,
    next: &RgbaImage,
    old: &[u64],
    new: &[u64],
    top: usize,
    bottom: usize,
) -> Option<i64> {
    let end = old.len() - bottom;
    let body_height = end - top;
    let min_overlap = (body_height / 5).max(4).min(body_height);
    let mut positions = HashMap::<u64, Vec<usize>>::new();
    for (y, hash) in old.iter().enumerate().take(end).skip(top) {
        positions.entry(*hash).or_default().push(y);
    }
    let anchors = (top..end)
        .filter(|y| {
            positions
                .get(&new[*y])
                .is_some_and(|matches| matches.len() <= 32)
        })
        .collect::<Vec<_>>();
    let mut candidates = BTreeSet::new();
    for y in anchors.iter().step_by((anchors.len() / 12).max(1)).take(16) {
        for old_y in &positions[&new[*y]] {
            let delta = *old_y as i64 - *y as i64;
            if delta != 0 && delta.unsigned_abs() as usize + min_overlap <= body_height {
                candidates.insert(delta);
            }
        }
    }
    let mut scores = candidates
        .into_iter()
        .filter_map(|delta| {
            let score = overlap_error(previous, next, top, end, delta);
            (score < 4.0).then_some((delta, score))
        })
        .collect::<Vec<_>>();
    scores.sort_by(|a, b| {
        a.1.total_cmp(&b.1)
            .then(a.0.unsigned_abs().cmp(&b.0.unsigned_abs()))
    });
    let best = scores.first()?;
    if scores
        .get(1)
        .is_some_and(|second| (second.1 - best.1).abs() < 0.05)
    {
        return None;
    }
    Some(best.0)
}

fn overlap_error(
    previous: &RgbaImage,
    next: &RgbaImage,
    top: usize,
    end: usize,
    delta: i64,
) -> f64 {
    let offset = delta.unsigned_abs() as usize;
    let mut sum = 0u64;
    let mut count = 0u64;
    for y in (top..end - offset).step_by(2) {
        let (old_y, new_y) = if delta > 0 {
            (y + offset, y)
        } else {
            (y, y + offset)
        };
        let old_bg = previous.get_pixel(matching_columns(previous).start, old_y as u32);
        let new_bg = next.get_pixel(matching_columns(next).start, new_y as u32);
        for x in matching_columns(previous).step_by(4) {
            let a = previous.get_pixel(x, old_y as u32);
            let b = next.get_pixel(x, new_y as u32);
            // Check the complete overlap, weighting text/detail over blank margins.
            if (0..3).any(|c| {
                a[c].abs_diff(old_bg[c]) > 12
                    || b[c].abs_diff(new_bg[c]) > 12
                    || a[c].abs_diff(b[c]) > 8
            }) {
                sum += (0..3).map(|c| a[c].abs_diff(b[c]) as u64).sum::<u64>();
                count += 3;
            }
        }
    }
    if count == 0 {
        f64::INFINITY
    } else {
        sum as f64 / count as f64
    }
}

fn changed_fraction(previous: &RgbaImage, next: &RgbaImage) -> f64 {
    let mut changed = 0usize;
    let mut samples = 0usize;
    for y in (0..previous.height()).step_by(4) {
        for x in matching_columns(previous).step_by(4) {
            samples += 1;
            if previous.get_pixel(x, y) != next.get_pixel(x, y) {
                changed += 1;
            }
        }
    }
    changed as f64 / samples.max(1) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(start: u32, header: u32, footer: u32) -> RgbaImage {
        RgbaImage::from_fn(64, 100, |x, y| {
            if y < header {
                return image::Rgba([20, 40, 160, 255]);
            }
            if y >= 100 - footer {
                return image::Rgba([160, 40, 20, 255]);
            }
            let row = start + y - header;
            image::Rgba([
                ((row * 13 + x * 7) % 251) as u8,
                ((row * 29 + x * 3) % 253) as u8,
                ((row * 3 + x * 11) % 249) as u8,
                255,
            ])
        })
    }

    #[test]
    fn joins_down_and_up_without_repeating_fixed_headers_or_footers() {
        let mut image = ScrollingImage::new(page(30, 10, 10)).unwrap();
        assert_eq!(image.push(page(50, 10, 10)), ScrollingStep::Appended(20));
        assert_eq!(image.dimensions(), (64, 120));
        assert_eq!(image.push(page(10, 10, 10)), ScrollingStep::Appended(20));
        assert_eq!(image.dimensions(), (64, 140));
        let output = image.image();
        assert_eq!(output.get_pixel(20, 0).0, [20, 40, 160, 255]);
        assert_eq!(output.get_pixel(20, 139).0, [160, 40, 20, 255]);
        assert_eq!(output.get_pixel(20, 10), page(10, 10, 10).get_pixel(20, 10));
    }

    #[test]
    fn does_not_duplicate_content_when_scrolling_inside_an_already_captured_area() {
        let mut image = ScrollingImage::new(page(0, 0, 0)).unwrap();
        assert_eq!(image.push(page(30, 0, 0)), ScrollingStep::Appended(30));
        assert_eq!(image.push(page(10, 0, 0)), ScrollingStep::Still);
        assert_eq!(image.dimensions(), (64, 130));
    }

    #[test]
    fn undo_keeps_the_last_segment_removed_until_the_page_changes() {
        let original = page(0, 10, 10);
        let mut image = ScrollingImage::new(original.clone()).unwrap();
        assert_eq!(image.push(page(20, 10, 10)), ScrollingStep::Appended(20));
        assert!(image.undo());
        assert_eq!(image.image(), original);
        assert_eq!(image.push(page(20, 10, 10)), ScrollingStep::Still);
        assert_eq!(image.dimensions(), (64, 100));
        assert_eq!(image.push(page(30, 10, 10)), ScrollingStep::Appended(30));
    }

    #[test]
    fn leaves_the_completed_image_intact_when_a_frame_cannot_be_matched() {
        let original = page(0, 0, 0);
        let mut image = ScrollingImage::new(original.clone()).unwrap();
        assert_eq!(
            image.push(RgbaImage::from_pixel(
                64,
                100,
                image::Rgba([255, 255, 255, 255])
            )),
            ScrollingStep::Failed
        );
        assert_eq!(image.image(), original);
    }

    #[test]
    fn static_or_ambiguous_blank_frames_do_not_append_rows() {
        let blank = RgbaImage::from_pixel(64, 100, image::Rgba([255, 255, 255, 255]));
        let mut image = ScrollingImage::new(blank.clone()).unwrap();
        assert_eq!(image.push(blank), ScrollingStep::Still);
        assert!(!image.can_undo());
    }

    #[test]
    fn creates_a_bounded_preview_without_changing_the_full_image_dimensions() {
        let mut image = ScrollingImage::new(page(0, 0, 0)).unwrap();
        image.push(page(40, 0, 0));
        let preview = image.thumbnail(20, 30);
        assert!(preview.width() <= 20 && preview.height() <= 30);
        assert_eq!(image.dimensions(), (64, 140));
    }

    #[test]
    fn undo_is_not_reversed_by_an_animated_fixed_header() {
        let original = page(0, 10, 10);
        let mut image = ScrollingImage::new(original.clone()).unwrap();
        image.push(page(20, 10, 10));
        assert!(image.undo());
        let mut animated = page(20, 10, 10);
        for y in 0..10 {
            for x in 0..64 {
                animated.put_pixel(x, y, image::Rgba([80, 80, 80, 255]));
            }
        }
        assert_eq!(image.push(animated), ScrollingStep::Still);
        assert_eq!(image.image(), original);
    }

    #[test]
    fn changing_scrollbar_position_does_not_prevent_content_matching() {
        let frame = |start: u32| {
            RgbaImage::from_fn(320, 100, |x, y| {
                if x >= 304 {
                    return image::Rgba([if y.abs_diff(start) < 10 { 60 } else { 240 }; 4]);
                }
                image::Rgba([
                    ((start + y) * 13 % 251) as u8,
                    ((start + y) * 29 % 253) as u8,
                    ((x * 7 + start + y) % 249) as u8,
                    255,
                ])
            })
        };
        let mut image = ScrollingImage::new(frame(0)).unwrap();
        assert_eq!(image.push(frame(20)), ScrollingStep::Appended(20));
        assert_eq!(image.dimensions(), (320, 120));
    }

    #[test]
    fn rejects_ambiguous_repeated_rows_without_changing_the_result() {
        let original = RgbaImage::from_fn(64, 100, |x, y| {
            image::Rgba([(y % 10 * 20) as u8, (x * 3) as u8, 70, 255])
        });
        let mut next = original.clone();
        for x in 0..64 {
            next.put_pixel(x, 96, image::Rgba([255, 255, 255, 255]));
        }
        let mut image = ScrollingImage::new(original.clone()).unwrap();
        assert_eq!(image.push(next), ScrollingStep::Failed);
        assert_eq!(image.image(), original);
    }

    #[test]
    fn enforces_height_and_pixel_limits_before_allocating_a_long_result() {
        assert!(!within_limits(0, 100));
        assert!(within_limits(2000, 25_000));
        assert!(!within_limits(2001, 25_000));
        assert!(ScrollingImage::new(RgbaImage::new(1, 30_001)).is_err());
        let frame = |start: u32| {
            RgbaImage::from_fn(8, 10_000, |x, y| {
                let row = start + y;
                image::Rgba([
                    ((row * 13 + x * 7) % 251) as u8,
                    ((row * 29 + x * 3) % 253) as u8,
                    ((row * 3 + x * 11) % 249) as u8,
                    255,
                ])
            })
        };
        let mut image = ScrollingImage::new(frame(0)).unwrap();
        for start in [5000, 10_000, 15_000] {
            assert_eq!(image.push(frame(start)), ScrollingStep::Appended(5000));
        }
        assert_eq!(image.push(frame(20_000)), ScrollingStep::Limit);
        assert_eq!(image.dimensions(), (8, 30_000));
        assert_eq!(image.push(frame(25_000)), ScrollingStep::Limit);
        assert_eq!(image.dimensions(), (8, 30_000));
        assert!(image.undo());
        assert_eq!(image.dimensions(), (8, 25_000));
    }

    #[test]
    fn crops_the_last_segment_at_a_configured_height_without_a_gap() {
        let mut image = ScrollingImage::with_max_height(page(0, 0, 0), 125).unwrap();
        assert_eq!(image.push(page(40, 0, 0)), ScrollingStep::Limit);
        assert_eq!(image.dimensions().1, 125);
        let expected = page(25, 0, 0);
        assert_eq!(image.image().get_pixel(8, 124), expected.get_pixel(8, 99));
        assert!(image.undo());
        assert_eq!(image.dimensions().1, 100);
        assert!(!image.at_limit());
    }
}
