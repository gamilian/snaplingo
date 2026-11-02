use crate::domain::capture::LogicalRect;

pub(crate) fn interface_scale(mode: &str, display: Option<&LogicalRect>) -> f64 {
    if let Ok(percent) = mode.parse::<f64>() {
        return (percent / 100.0).clamp(1.0, 2.0);
    }
    display.map_or(1.0, |display| {
        (display.width / 1920.0)
            .max(display.height / 1080.0)
            .max(1.0)
            .powf(0.6)
            .min(3.0)
    })
}
