//! Per-pixel alpha presentation for capture and pinned-image windows.
use windows::Win32::{
    Foundation::{COLORREF, HANDLE, HWND, POINT, SIZE},
    Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ,
    },
    UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, UpdateLayeredWindow, GWL_EXSTYLE, ULW_ALPHA,
        WS_EX_LAYERED,
    },
};
use winit::{
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::Window,
};

pub(crate) struct LayeredPresenter {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    bits: *mut u8,
    size: (u32, u32),
}
impl LayeredPresenter {
    pub(crate) fn new(window: &Window) -> Result<Self, String> {
        let hwnd = hwnd(window)?;
        unsafe {
            let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED.0 as isize);
            let dc = CreateCompatibleDC(HDC(0));
            if dc.0 == 0 {
                return Err("Failed to create the layered-window drawing context".into());
            }
            Ok(Self {
                dc,
                bitmap: HBITMAP(0),
                previous: HGDIOBJ(0),
                bits: std::ptr::null_mut(),
                size: (0, 0),
            })
        }
    }
    fn resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        if self.size == (width, height) {
            return Ok(());
        }
        if width == 0 || height == 0 || width > i32::MAX as u32 || height > i32::MAX as u32 {
            return Err("Invalid layered-window image dimensions".into());
        }
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
        let mut bits = std::ptr::null_mut();
        let bitmap =
            unsafe { CreateDIBSection(self.dc, &info, DIB_RGB_COLORS, &mut bits, HANDLE(0), 0) }
                .map_err(|e| e.to_string())?;
        unsafe {
            let old = SelectObject(self.dc, bitmap);
            if self.bitmap.0 == 0 {
                self.previous = old;
            } else {
                let _ = DeleteObject(self.bitmap);
            }
        }
        self.bitmap = bitmap;
        self.bits = bits.cast();
        self.size = (width, height);
        Ok(())
    }
    pub(crate) fn present(
        &mut self,
        window: &Window,
        frame: &snaplingo_render::Frame,
    ) -> Result<(), String> {
        self.present_rgba(window, frame.width(), frame.height(), frame.rgba())
    }
    /// Accepts premultiplied RGBA and converts it to Win32's top-down BGRA DIB.
    pub(crate) fn present_rgba(
        &mut self,
        window: &Window,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> Result<(), String> {
        let length = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .ok_or("Layered image dimensions overflow")?;
        if pixels.len() != length {
            return Err("Layered image pixels do not match its dimensions".into());
        }
        self.resize(width, height)?;
        let destination = unsafe { std::slice::from_raw_parts_mut(self.bits, length) };
        for (rgba, bgra) in pixels.chunks_exact(4).zip(destination.chunks_exact_mut(4)) {
            bgra.copy_from_slice(&[rgba[2], rgba[1], rgba[0], rgba[3]]);
        }
        let size = SIZE {
            cx: width as i32,
            cy: height as i32,
        };
        let origin = POINT::default();
        let blend = BLENDFUNCTION {
            BlendOp: 0,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: 1,
        };
        unsafe {
            UpdateLayeredWindow(
                hwnd(window)?,
                HDC(0),
                None,
                Some(&size),
                self.dc,
                Some(&origin),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            )
        }
        .map_err(|e| e.to_string())
    }
}
impl Drop for LayeredPresenter {
    fn drop(&mut self) {
        unsafe {
            if self.bitmap.0 != 0 {
                SelectObject(self.dc, self.previous);
                let _ = DeleteObject(self.bitmap);
            }
            let _ = DeleteDC(self.dc);
        }
    }
}
fn hwnd(window: &Window) -> Result<HWND, String> {
    match window.window_handle().map_err(|e| e.to_string())?.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(HWND(handle.hwnd.get())),
        _ => Err("A Win32 window handle is required for layered presentation".into()),
    }
}
