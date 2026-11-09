//! Common navigation pixels for the independently owned dynamic settings pages.
use crate::*;

pub(crate) fn shell_frame(snapshot: &Snapshot, page: Page, size: Size) -> Scene {
    let mut ui = SettingsUi::new(snapshot.clone());
    ui.page = page;
    let p = palette(snapshot.theme, snapshot.dark);
    let mut paint = Paint::new(size, p, None, None);
    paint.fill(r(0., 0., size.width, size.height), 0., p.canvas);
    let narrow = size.width <= 700.;
    let rail = match snapshot.theme {
        Theme::Paper => 0.,
        Theme::Studio => 68.,
        Theme::Native if narrow => 65.,
        Theme::Native => 202.,
    };
    let top = if snapshot.theme == Theme::Paper {
        68.
    } else {
        0.
    };
    ui.navigation(&mut paint, rail, top, narrow);
    paint.scene.content = r(
        rail,
        top,
        (size.width - rail).max(1.),
        (size.height - top).max(1.),
    );
    paint.scene.viewport = paint.scene.content;
    paint.scene
}
