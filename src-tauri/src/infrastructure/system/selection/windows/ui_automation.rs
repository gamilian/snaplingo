use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation8, IUIAutomation2, IUIAutomationTextPattern, UIA_TextPatternId,
};
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

pub(super) fn read_selected_text() -> windows::core::Result<String> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)?;
        let _com = ComApartment;
        let automation: IUIAutomation2 =
            CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
        automation.SetConnectionTimeout(200)?;
        automation.SetTransactionTimeout(200)?;
        let foreground = GetForegroundWindow();
        let root = automation.ElementFromHandle(foreground)?;
        let mut element = automation.GetFocusedElement()?;
        let walker = automation.ControlViewWalker()?;

        // A focused link or child control can expose its selection through its
        // document ancestor. Stop at the foreground window, never the desktop.
        for _ in 0..8 {
            if element.CurrentIsPassword()?.as_bool() || GetForegroundWindow() != foreground {
                return Ok(String::new());
            }
            if let Ok(pattern) =
                element.GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId)
            {
                let ranges = pattern.GetSelection()?;
                let mut selected = Vec::new();
                for index in 0..ranges.Length()? {
                    // GetText on a selection range preserves line breaks and
                    // returns empty for a caret. Never read Value/DocumentRange.
                    let text = ranges.GetElement(index)?.GetText(-1)?.to_string();
                    if !text.trim().is_empty() {
                        selected.push(text);
                    }
                }
                if GetForegroundWindow() != foreground {
                    return Ok(String::new());
                }
                return Ok(selected.join("\n"));
            }
            if automation.CompareElements(&element, &root)?.as_bool() {
                break;
            }
            element = walker.GetParentElement(&element)?;
        }
        Ok(String::new())
    }
}

struct ComApartment;

impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}
