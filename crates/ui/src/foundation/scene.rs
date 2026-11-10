//! Pure drawing output. Interaction, IME and scrolling belong to each view frame.
use super::{DrawCommand, Size};

#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    pub size: Size,
    pub commands: Vec<DrawCommand>,
}
impl Scene {
    pub fn scaled(mut self, scale: f32) -> Self {
        self.size.width *= scale;
        self.size.height *= scale;
        for command in &mut self.commands {
            match command {
                DrawCommand::Image { rect, .. } | DrawCommand::PushClip(rect) => {
                    *rect = rect.scaled(scale)
                }
                DrawCommand::FillRect { rect, radius, .. } => {
                    *rect = rect.scaled(scale);
                    *radius *= scale;
                }
                DrawCommand::StrokeRect {
                    rect,
                    radius,
                    width,
                    ..
                } => {
                    *rect = rect.scaled(scale);
                    *radius *= scale;
                    *width *= scale;
                }
                DrawCommand::Line {
                    from, to, width, ..
                } => {
                    from.x *= scale;
                    from.y *= scale;
                    to.x *= scale;
                    to.y *= scale;
                    *width *= scale;
                }
                DrawCommand::Text { rect, size, .. } => {
                    *rect = rect.scaled(scale);
                    *size *= scale;
                }
                DrawCommand::TextBlock {
                    rect,
                    size,
                    line_height,
                    ..
                } => {
                    *rect = rect.scaled(scale);
                    *size *= scale;
                    *line_height *= scale;
                }
                DrawCommand::PopClip => {}
            }
        }
        self
    }
}
