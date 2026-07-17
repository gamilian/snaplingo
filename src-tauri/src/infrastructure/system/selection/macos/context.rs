use crate::domain::{FrontmostApp, SelectionContext};
use std::sync::atomic::{AtomicBool, Ordering};

static ACCESSIBILITY_PROMPT_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn accessibility_permission_granted(prompt: bool) -> bool {
    use std::ffi::c_void;
    use std::os::raw::{c_long, c_uchar};

    type Boolean = c_uchar;
    type CFAllocatorRef = *const c_void;
    type CFDictionaryRef = *const c_void;
    type CFStringRef = *const c_void;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> c_uchar;
        fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> Boolean;
        static kAXTrustedCheckOptionPrompt: CFStringRef;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFDictionaryCreate(
            allocator: CFAllocatorRef,
            keys: *const *const c_void,
            values: *const *const c_void,
            num_values: c_long,
            key_callbacks: *const c_void,
            value_callbacks: *const c_void,
        ) -> CFDictionaryRef;
        fn CFRelease(cf: *const c_void);
        static kCFBooleanTrue: *const c_void;
    }

    if !prompt {
        return unsafe { AXIsProcessTrusted() != 0 };
    }

    unsafe {
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let options = CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            std::ptr::null(),
            std::ptr::null(),
        );

        if options.is_null() {
            return AXIsProcessTrusted() != 0;
        }

        let trusted = AXIsProcessTrustedWithOptions(options) != 0;
        CFRelease(options);
        trusted
    }
}

pub fn request_accessibility_permission() -> bool {
    if accessibility_permission_granted(false) {
        return true;
    }
    if ACCESSIBILITY_PROMPT_REQUESTED.swap(true, Ordering::AcqRel) {
        return false;
    }
    accessibility_permission_granted(true)
}

pub fn selection_accessibility_permission_error() -> String {
    format!(
        "{} {}",
        "划词翻译需要 macOS 辅助功能权限。SnapLingo 已通过 macOS 系统授权流程发起请求；如未自动打开，请在 系统设置 > 隐私与安全性 > 辅助功能 中允许 SnapLingo，然后重新触发或重启应用。",
        macos_accessibility_runtime_context(),
    )
}

pub fn frontmost_context(self_bundle_id: Option<String>) -> SelectionContext {
    SelectionContext {
        frontmost_app: frontmost_app(),
        self_bundle_id,
    }
}

fn frontmost_app() -> Option<FrontmostApp> {
    let bundle_id = run_osascript(r#"id of app (path to frontmost application as text)"#);
    let name = run_osascript(r#"name of app (path to frontmost application as text)"#);

    if bundle_id.is_none() && name.is_none() {
        return None;
    }

    Some(FrontmostApp { bundle_id, name })
}

fn run_osascript(script: &str) -> Option<String> {
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

pub fn read_accessibility_selected_text() -> Result<Option<String>, String> {
    use std::ffi::{c_void, CStr};
    use std::os::raw::{c_char, c_int, c_long};

    type AXError = c_int;
    type AXUIElementRef = *const c_void;
    type Boolean = std::os::raw::c_uchar;
    type CFIndex = c_long;
    type CFStringEncoding = u32;
    type CFStringRef = *const c_void;
    type CFTypeID = c_long;
    type CFTypeRef = *const c_void;

    const AX_ERROR_SUCCESS: AXError = 0;
    const K_CF_STRING_ENCODING_UTF8: CFStringEncoding = 0x0800_0100;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXUIElementCreateSystemWide() -> AXUIElementRef;
        fn AXUIElementCopyAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            value: *mut CFTypeRef,
        ) -> AXError;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFGetTypeID(cf: CFTypeRef) -> CFTypeID;
        fn CFRelease(cf: CFTypeRef);
        fn CFStringCreateWithCString(
            alloc: *const c_void,
            c_str: *const c_char,
            encoding: CFStringEncoding,
        ) -> CFStringRef;
        fn CFStringGetCString(
            the_string: CFStringRef,
            buffer: *mut c_char,
            buffer_size: CFIndex,
            encoding: CFStringEncoding,
        ) -> Boolean;
        fn CFStringGetLength(the_string: CFStringRef) -> CFIndex;
        fn CFStringGetMaximumSizeForEncoding(
            length: CFIndex,
            encoding: CFStringEncoding,
        ) -> CFIndex;
        fn CFStringGetTypeID() -> CFTypeID;
    }

    unsafe fn copy_attribute_value(
        element: AXUIElementRef,
        attribute: CFStringRef,
    ) -> Result<Option<CFTypeRef>, String> {
        let mut value = std::ptr::null();
        let error = unsafe { AXUIElementCopyAttributeValue(element, attribute, &mut value) };
        if error == AX_ERROR_SUCCESS && !value.is_null() {
            return Ok(Some(value));
        }
        if error == AX_ERROR_SUCCESS {
            return Ok(None);
        }
        Err(format!("AX attribute read failed with code {}", error))
    }

    unsafe fn cf_string_from_static_bytes(bytes: &'static [u8]) -> Result<CFStringRef, String> {
        let value = unsafe {
            CFStringCreateWithCString(
                std::ptr::null(),
                bytes.as_ptr() as *const c_char,
                K_CF_STRING_ENCODING_UTF8,
            )
        };
        if value.is_null() {
            return Err("Failed to create Accessibility attribute name".to_string());
        }
        Ok(value)
    }

    unsafe fn cf_string_to_string(value: CFTypeRef) -> Result<Option<String>, String> {
        if unsafe { CFGetTypeID(value) } != unsafe { CFStringGetTypeID() } {
            return Ok(None);
        }

        let string_ref = value as CFStringRef;
        let length = unsafe { CFStringGetLength(string_ref) };
        let max_size =
            unsafe { CFStringGetMaximumSizeForEncoding(length, K_CF_STRING_ENCODING_UTF8) } + 1;
        if max_size <= 0 {
            return Ok(Some(String::new()));
        }

        let mut buffer = vec![0; max_size as usize];
        let copied = unsafe {
            CFStringGetCString(
                string_ref,
                buffer.as_mut_ptr(),
                max_size,
                K_CF_STRING_ENCODING_UTF8,
            )
        };
        if copied == 0 {
            return Err("Failed to convert AX selected text to UTF-8".to_string());
        }

        Ok(Some(
            unsafe { CStr::from_ptr(buffer.as_ptr()) }
                .to_string_lossy()
                .into_owned(),
        ))
    }

    unsafe fn copy_selected_text_from_element(
        element: AXUIElementRef,
        selected_text_attribute: CFStringRef,
    ) -> Result<Option<String>, String> {
        let Some(value) = (unsafe { copy_attribute_value(element, selected_text_attribute)? })
        else {
            return Ok(None);
        };

        let text = unsafe { cf_string_to_string(value) };
        unsafe { CFRelease(value) };
        text
    }

    if !accessibility_permission_granted(false) {
        return Ok(None);
    }

    let system_wide = unsafe { AXUIElementCreateSystemWide() };
    if system_wide.is_null() {
        return Ok(None);
    }

    let focused_ui_element_attribute =
        unsafe { cf_string_from_static_bytes(b"AXFocusedUIElement\0")? };
    let focused_element =
        unsafe { copy_attribute_value(system_wide, focused_ui_element_attribute) };
    unsafe { CFRelease(focused_ui_element_attribute) };
    unsafe { CFRelease(system_wide) };

    let Some(focused_element) = focused_element? else {
        return Ok(None);
    };

    let selected_text_attribute = unsafe { cf_string_from_static_bytes(b"AXSelectedText\0")? };
    let selected_text =
        unsafe { copy_selected_text_from_element(focused_element, selected_text_attribute) };
    unsafe { CFRelease(selected_text_attribute) };
    unsafe { CFRelease(focused_element) };
    selected_text
}

fn macos_accessibility_runtime_context() -> String {
    let executable = std::env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|e| format!("unknown ({e})"));

    let bundle = std::env::current_exe()
        .ok()
        .and_then(|path| macos_app_bundle_path(&path))
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "not running from a .app bundle".to_string());

    format!("当前运行路径：{executable}；App Bundle：{bundle}。如果这里不是你在辅助功能列表中授权的同一个 SnapLingo，请授权当前运行的应用。")
}

fn macos_app_bundle_path(executable_path: &std::path::Path) -> Option<std::path::PathBuf> {
    executable_path
        .ancestors()
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
        .map(std::path::Path::to_path_buf)
}

#[cfg(test)]
mod macos_selection_context_tests {
    use super::*;

    #[test]
    fn app_bundle_path_detects_bundle_ancestor() {
        let executable_path =
            std::path::Path::new("/Applications/SnapLingo.app/Contents/MacOS/snaplingo");

        assert_eq!(
            macos_app_bundle_path(executable_path).as_deref(),
            Some(std::path::Path::new("/Applications/SnapLingo.app"))
        );
    }

    #[test]
    fn permission_error_includes_runtime_context() {
        let err = selection_accessibility_permission_error();

        assert!(err.contains("当前运行路径："));
        assert!(err.contains("App Bundle："));
    }
}
