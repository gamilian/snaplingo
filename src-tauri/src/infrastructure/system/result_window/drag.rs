#[tauri::command]
pub fn start_result_window_drag(window: tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let _main_thread = objc2::MainThreadMarker::new()
            .ok_or("Result window dragging must run on the main thread")?;
        let native = window.ns_window().map_err(|error| error.to_string())?;
        let native = unsafe { native.cast::<objc2_app_kit::NSWindow>().as_ref() }
            .ok_or("Result window has no native NSWindow")?;
        super::drag_macos::start_dragging(native)
    }

    #[cfg(not(target_os = "macos"))]
    window.start_dragging().map_err(|error| error.to_string())
}
