//! Native dialogs used by the embedded shared screenshot preferences.
use std::path::PathBuf;

use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSAlert, NSAlertFirstButtonReturn, NSColor, NSColorPanel, NSColorSpace, NSColorWell,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

pub(crate) async fn select_directory(initial: Option<PathBuf>) -> Result<Option<PathBuf>, String> {
    let future = crate::infrastructure::ui_thread::run("select screenshot directory", move || {
        let mut dialog = rfd::AsyncFileDialog::new()
            .set_title("选择截图保存位置")
            .set_can_create_directories(true);
        if let Some(directory) = initial {
            dialog = dialog.set_directory(directory);
        }
        // rfd chooses the native main window as its sheet parent. Creating its
        // future on the UI thread also makes its no-window modal fallback safe.
        Ok(dialog.pick_folder())
    })
    .await
    .map_err(|error| error.to_string())?;
    Ok(future.await.map(|file| file.path().to_path_buf()))
}

/// Call on the AppKit thread. Cancellation leaves the saved preference intact.
pub(crate) fn choose_color(initial: [u8; 4]) -> Result<Option<[u8; 4]>, String> {
    let mtm = MainThreadMarker::new().ok_or("颜色对话框必须在窗口主线程打开")?;
    let alert = NSAlert::new(mtm);
    alert.setMessageText(&NSString::from_str("自定义颜色"));
    alert.setInformativeText(&NSString::from_str("点击色块选择颜色和透明度，然后确认。"));
    alert.addButtonWithTitle(&NSString::from_str("确认"));
    alert.addButtonWithTitle(&NSString::from_str("取消"));
    let well = NSColorWell::initWithFrame(
        mtm.alloc(),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(250.0, 44.0)),
    );
    let [r, g, b, a] = initial.map(|channel| f64::from(channel) / 255.0);
    well.setColor(&NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, a));
    alert.setAccessoryView(Some(&well));
    let panel = NSColorPanel::sharedColorPanel(mtm);
    let previous_alpha = panel.showsAlpha();
    let previous_color = panel.color();
    let previous_visibility = panel.isVisible();
    let previous_modal = panel.worksWhenModal();
    panel.setShowsAlpha(true);
    panel.setWorksWhenModal(true);
    let result = if alert.runModal() == NSAlertFirstButtonReturn {
        well.color()
            .colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())
            .map(|color| {
                [
                    color.redComponent(),
                    color.greenComponent(),
                    color.blueComponent(),
                    color.alphaComponent(),
                ]
                .map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8)
            })
            .ok_or("无法读取所选颜色")
            .map(Some)
    } else {
        Ok(None)
    };
    well.deactivate();
    if !previous_visibility {
        panel.orderOut(None);
    }
    panel.setShowsAlpha(previous_alpha);
    panel.setWorksWhenModal(previous_modal);
    panel.setColor(&previous_color);
    result.map_err(Into::into)
}
