#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Presentation {
    pub zoom: f64,
    pub opacity: f64,
    pub rotation: u16,
    pub flip_x: bool,
    pub flip_y: bool,
    pub grayscale: bool,
    pub inverted: bool,
    pub thumbnail: bool,
}

impl Default for Presentation {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            opacity: 1.0,
            rotation: 0,
            flip_x: false,
            flip_y: false,
            grayscale: false,
            inverted: false,
            thumbnail: false,
        }
    }
}

impl Presentation {
    pub fn zoom_by(&mut self, step: f64) {
        self.thumbnail = false;
        self.zoom = ((self.zoom + step).clamp(0.1, 8.0) * 100.0).round() / 100.0;
    }

    pub fn opacity_by(&mut self, step: f64) {
        self.opacity = ((self.opacity * 255.0).round() + step).clamp(12.0, 255.0) / 255.0;
    }

    pub fn export_size(&self, width: u32, height: u32) -> ImageSize {
        let (width, height) = (width.max(1) as f64, height.max(1) as f64);
        let scale = if self.thumbnail {
            (220.0 / width).min(160.0 / height).min(1.0)
        } else {
            let initial = (900.0 / width).min(700.0 / height).min(1.0);
            (initial * self.zoom.clamp(0.1, 8.0))
                .min(4096.0 / width)
                .min(4096.0 / height)
        };
        ImageSize {
            width: (width * scale)
                .round()
                .max(if self.thumbnail { 80.0 } else { 1.0 }) as u32,
            height: (height * scale)
                .round()
                .max(if self.thumbnail { 60.0 } else { 1.0 }) as u32,
        }
    }

    pub fn window_size(&self, width: u32, height: u32) -> (f64, f64) {
        let size = self.export_size(width, height);
        if self.rotation == 90 || self.rotation == 270 {
            (size.height as f64, size.width as f64)
        } else {
            (size.width as f64, size.height as f64)
        }
    }

    pub fn sample_point(&self, point: (f64, f64), width: u32, height: u32) -> (u32, u32) {
        let (display_width, display_height) = self.window_size(width, height);
        let (x, y) = (
            (point.0 / display_width).clamp(0.0, 1.0),
            (point.1 / display_height).clamp(0.0, 1.0),
        );
        let (mut x, mut y) = match self.rotation {
            90 => (y, 1.0 - x),
            180 => (1.0 - x, 1.0 - y),
            270 => (1.0 - y, x),
            _ => (x, y),
        };
        if self.flip_x {
            x = 1.0 - x;
        }
        if self.flip_y {
            y = 1.0 - y;
        }
        (
            ((x * width as f64).floor() as u32).min(width.saturating_sub(1)),
            ((y * height as f64).floor() as u32).min(height.saturating_sub(1)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_keeps_image_limits_and_rotated_window_dimensions() {
        let mut view = Presentation::default();
        assert_eq!(view.window_size(1800, 900), (900.0, 450.0));
        view.zoom_by(100.0);
        assert_eq!(view.zoom, 8.0);
        assert_eq!(view.window_size(1800, 900), (4096.0, 2048.0));
        view.rotation = 90;
        assert_eq!(view.window_size(1800, 900), (2048.0, 4096.0));
        assert_eq!(view.export_size(1800, 900).width, 4096);
        view.zoom_by(-100.0);
        assert_eq!(view.zoom, 0.1);
    }

    #[test]
    fn thumbnail_and_opacity_match_existing_pin_interactions() {
        let mut view = Presentation {
            thumbnail: true,
            ..Presentation::default()
        };
        assert_eq!(view.window_size(20, 10), (80.0, 60.0));
        view.opacity_by(-300.0);
        assert_eq!(view.opacity, 12.0 / 255.0);
        view.opacity_by(300.0);
        assert_eq!(view.opacity, 1.0);
        view.zoom_by(0.1);
        assert!(!view.thumbnail);
        assert_eq!(view.zoom, 1.1);
    }

    #[test]
    fn inspection_maps_rotation_and_flips_back_to_original_pixels() {
        let mut view = Presentation::default();
        assert_eq!(view.sample_point((20.0, 10.0), 100, 60), (20, 10));
        view.rotation = 90;
        assert_eq!(view.sample_point((10.0, 20.0), 100, 60), (20, 50));
        view.flip_x = true;
        assert_eq!(view.sample_point((10.0, 20.0), 100, 60), (80, 50));
        assert_eq!(view.sample_point((-100.0, 999.0), 100, 60), (0, 59));
    }
}
