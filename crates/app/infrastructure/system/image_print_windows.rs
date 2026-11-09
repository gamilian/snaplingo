//! Native image printing; no image-viewer file association is required.
fn fit(image: (u32, u32), page: (i32, i32)) -> Result<(i32, i32, i32, i32), String> {
    if image.0 == 0 || image.1 == 0 || page.0 <= 0 || page.1 <= 0 {
        return Err("The image or printable page has no area".into());
    }
    let scale = (page.0 as f64 / image.0 as f64).min(page.1 as f64 / image.1 as f64);
    let width = (image.0 as f64 * scale).round().max(1.) as i32;
    let height = (image.1 as f64 * scale).round().max(1.) as i32;
    Ok(((page.0 - width) / 2, (page.1 - height) / 2, width, height))
}

#[cfg(target_os = "windows")]
pub(crate) fn print(path: &std::path::Path, owner: &winit::window::Window) -> Result<bool, String> {
    use windows::{
        core::w,
        Win32::{
            Foundation::{GlobalFree, HWND},
            Graphics::Gdi::*,
            Storage::Xps::{AbortDoc, EndDoc, EndPage, StartDocW, StartPage, DOCINFOW},
            UI::Controls::Dialogs::*,
        },
    };
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    struct Printer(PRINTDLGW);
    impl Drop for Printer {
        fn drop(&mut self) {
            unsafe {
                if self.0.hDC.0 != 0 {
                    let _ = DeleteDC(self.0.hDC);
                }
                if !self.0.hDevMode.0.is_null() {
                    let _ = GlobalFree(self.0.hDevMode);
                }
                if !self.0.hDevNames.0.is_null() {
                    let _ = GlobalFree(self.0.hDevNames);
                }
            }
        }
    }
    let handle = match owner.window_handle().map_err(|e| e.to_string())?.as_raw() {
        RawWindowHandle::Win32(handle) => HWND(handle.hwnd.get()),
        _ => return Err("Printing requires a Win32 window".into()),
    };
    let mut image = image::open(path).map_err(|e| e.to_string())?.to_rgba8();
    let (width, height) = image.dimensions();
    if width > i32::MAX as u32 || height > i32::MAX as u32 {
        return Err("The image is too large to print".into());
    }
    // Printers consume opaque BGRA. Composite transparent pixels onto paper.
    for pixel in image.as_mut().chunks_exact_mut(4) {
        let a = u32::from(pixel[3]);
        let channel = |value: u8| ((u32::from(value) * a + 255 * (255 - a) + 127) / 255) as u8;
        let (r, g, b) = (channel(pixel[0]), channel(pixel[1]), channel(pixel[2]));
        pixel.copy_from_slice(&[b, g, r, 255]);
    }
    let mut printer = Printer(PRINTDLGW {
        lStructSize: std::mem::size_of::<PRINTDLGW>() as u32,
        hwndOwner: handle,
        Flags: PD_RETURNDC | PD_NOPAGENUMS | PD_NOSELECTION | PD_USEDEVMODECOPIESANDCOLLATE,
        nCopies: 1,
        ..Default::default()
    });
    if !unsafe { PrintDlgW(&mut printer.0) }.as_bool() {
        let error = unsafe { CommDlgExtendedError() };
        return if error.0 == 0 {
            Ok(false)
        } else {
            Err(format!(
                "Windows could not open the print dialog: {}",
                error.0
            ))
        };
    }
    let dc = printer.0.hDC;
    let (x, y, destination_width, destination_height) = fit((width, height), unsafe {
        (GetDeviceCaps(dc, HORZRES), GetDeviceCaps(dc, VERTRES))
    })?;
    let document = DOCINFOW {
        cbSize: std::mem::size_of::<DOCINFOW>() as i32,
        lpszDocName: w!("SnapLingo screenshot"),
        ..Default::default()
    };
    if unsafe { StartDocW(dc, &document) } <= 0 {
        return Err("Windows could not start the print job".into());
    }
    struct Job(HDC, bool);
    impl Drop for Job {
        fn drop(&mut self) {
            if !self.1 {
                unsafe {
                    AbortDoc(self.0);
                }
            }
        }
    }
    let mut job = Job(dc, false);
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    for _ in 0..printer.0.nCopies.max(1) {
        if unsafe { StartPage(dc) } <= 0 {
            return Err("Windows could not start the printed page".into());
        }
        unsafe {
            SetStretchBltMode(dc, HALFTONE);
            SetBrushOrgEx(dc, 0, 0, None);
        }
        let copied = unsafe {
            StretchDIBits(
                dc,
                x,
                y,
                destination_width,
                destination_height,
                0,
                0,
                width as i32,
                height as i32,
                Some(image.as_ptr().cast()),
                &info,
                DIB_RGB_COLORS,
                SRCCOPY,
            )
        };
        if copied <= 0 || unsafe { EndPage(dc) } <= 0 {
            return Err("Windows could not render the image to the printer".into());
        }
    }
    if unsafe { EndDoc(dc) } <= 0 {
        return Err("Windows could not finish the print job".into());
    }
    job.1 = true;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn centers_landscape_and_portrait_images_inside_printable_paper_without_distortion() {
        assert_eq!(fit((200, 100), (1000, 1500)).unwrap(), (0, 500, 1000, 500));
        assert_eq!(fit((100, 200), (1500, 1000)).unwrap(), (500, 0, 500, 1000));
        assert!(fit((0, 200), (1500, 1000)).is_err());
    }
}
