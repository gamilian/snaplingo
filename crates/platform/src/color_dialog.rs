//! Native color selection. The caller saves
//! only a confirmed result; cancellation leaves the existing value intact.
pub fn choose_color(initial: [u8; 4]) -> Result<Option<[u8; 4]>, String> {
    platform::choose_color(initial)
}

#[cfg(target_os = "macos")]
mod platform {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{
        NSAlert, NSAlertFirstButtonReturn, NSColor, NSColorPanel, NSColorSpace, NSColorWell,
    };
    use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

    pub(super) fn choose_color(initial: [u8; 4]) -> Result<Option<[u8; 4]>, String> {
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
        let accepted = alert.runModal() == NSAlertFirstButtonReturn;
        let result = if accepted {
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
}

#[cfg(target_os = "windows")]
mod platform {
    use windows_sys::Win32::UI::Controls::Dialogs::{
        ChooseColorW, CommDlgExtendedError, CC_ANYCOLOR, CC_FULLOPEN, CC_RGBINIT, CHOOSECOLORW,
    };
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow;

    pub(super) fn choose_color(initial: [u8; 4]) -> Result<Option<[u8; 4]>, String> {
        let mut custom = [0u32; 16];
        let mut options: CHOOSECOLORW = unsafe { std::mem::zeroed() };
        options.lStructSize = std::mem::size_of::<CHOOSECOLORW>() as u32;
        options.hwndOwner = unsafe { GetActiveWindow() };
        options.rgbResult =
            u32::from(initial[0]) | (u32::from(initial[1]) << 8) | (u32::from(initial[2]) << 16);
        options.lpCustColors = custom.as_mut_ptr();
        options.Flags = CC_ANYCOLOR | CC_FULLOPEN | CC_RGBINIT;
        if unsafe { ChooseColorW(&mut options) } != 0 {
            Ok(Some([
                options.rgbResult as u8,
                (options.rgbResult >> 8) as u8,
                (options.rgbResult >> 16) as u8,
                initial[3],
            ]))
        } else {
            let code = unsafe { CommDlgExtendedError() };
            if code == 0 {
                Ok(None)
            } else {
                Err(format!("无法打开颜色对话框：Windows {code:#x}"))
            }
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod platform {
    pub(super) fn choose_color(_: [u8; 4]) -> Result<Option<[u8; 4]>, String> {
        Err("当前平台尚未提供原生颜色选择器".into())
    }
}
