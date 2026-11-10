//! Opt-in native acceptance entrypoints. The harness links the application's
//! compiled modules; private state and window probes do not leave this module.
#![allow(dead_code, unused_variables)]
#[cfg(target_os = "macos")]
mod native_ui_workflows;
#[cfg(target_os = "macos")]
mod pinned_window;
#[cfg(target_os = "macos")]
mod references;
#[cfg(target_os = "macos")]
mod scrolling_controls;
#[cfg(target_os = "macos")]
mod scrolling_overlays;
#[cfg(target_os = "macos")]
mod selected_text;
#[cfg(target_os = "macos")]
mod support;
#[cfg(target_os = "macos")]
mod ui_thread;

/// A process-isolated acceptance scenario. Each native test target chooses one.
#[derive(Clone, Copy, Debug)]
pub enum Case {
    Application,
    Workflows,
    SelectedText,
    PinnedWindow,
    ScrollingControls,
    ScrollingOverlays,
    UiThread,
    GeneralReference,
    FeaturesReference,
    HotkeysReference,
}

/// Run one scenario on the process main thread. Assertion failures propagate to
/// its harness process. Unsupported platform scenarios report that explicitly.
pub fn run(case: Case) {
    #[cfg(target_os = "macos")]
    {
        objc2::MainThreadMarker::new()
            .expect("Native acceptance must run on the process main thread");
        match case {
            Case::Application => {
                if std::env::args().any(|arg| arg == "--permissions") {
                    crate::shell::macos::permission_smoke_test();
                } else {
                    crate::shell::macos::smoke_test();
                }
            }
            Case::Workflows => native_ui_workflows::run(),
            Case::SelectedText => selected_text::run(),
            Case::PinnedWindow => pinned_window::run(),
            Case::ScrollingControls => scrolling_controls::run(),
            Case::ScrollingOverlays => scrolling_overlays::run(),
            Case::UiThread => ui_thread::run(),
            Case::GeneralReference => references::general::run(),
            Case::FeaturesReference => references::features::run(),
            Case::HotkeysReference => references::hotkeys::run(),
        }
    }
    #[cfg(target_os = "windows")]
    match case {
        Case::Application => crate::shell::windows::smoke_test(),
        _ => println!("{case:?}: this AppKit acceptance scenario is macOS-only"),
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    println!("{case:?}: the complete native desktop host is unavailable on this platform");
}
