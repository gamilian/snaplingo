use enigo::{Enigo, MouseControllable};

use crate::application::CaptureCursorMover;

pub struct EnigoCaptureCursorMover;

impl CaptureCursorMover for EnigoCaptureCursorMover {
    fn move_relative(&self, delta_x: i32, delta_y: i32) {
        Enigo::new().mouse_move_relative(delta_x, delta_y);
    }
}
