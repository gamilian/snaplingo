use crate::infrastructure::system::selection::common::clipboard_transaction::ClipboardTextChange;

pub(super) struct ClipboardSnapshot {
    change_count: i64,
    formats: Vec<(u32, Vec<u8>)>,
}

impl ClipboardSnapshot {
    pub(super) fn change_count(&self) -> i64 {
        self.change_count
    }
}

#[cfg(target_os = "windows")]
pub(super) fn snapshot() -> Result<ClipboardSnapshot, String> {
    let _clipboard = open_clipboard()?;
    let mut supported = vec![1, 7, 13, 8, 17, 15, 16]; // ANSI text, OEM text, Unicode, DIB variants, HDROP, locale.
    for name in [
        "HTML Format",
        "PNG",
        "Rich Text Format",
        "Preferred DropEffect",
    ] {
        let format = clipboard_win::register_format(name)
            .ok_or_else(|| format!("Failed to register clipboard format {name}"))?;
        supported.push(format.get());
    }
    let mut formats = Vec::new();
    for format in supported {
        if clipboard_win::is_format_avail(format) {
            let mut data = Vec::new();
            clipboard_win::raw::get_vec(format, &mut data).map_err(|e| e.to_string())?;
            formats.push((format, data));
        }
    }
    if formats.is_empty() && clipboard_win::count_formats() != Some(0) {
        return Err("Clipboard contains no supported formats to restore".into());
    }
    Ok(ClipboardSnapshot {
        change_count: super::shortcut_copy::clipboard_change_count()?,
        formats,
    })
}

#[cfg(target_os = "windows")]
pub(super) fn restore(
    snapshot: &ClipboardSnapshot,
    change: &ClipboardTextChange,
) -> Result<(), String> {
    // Hold the native clipboard lock through the revision check and all writes.
    let _clipboard = open_clipboard()?;
    restore_with(
        snapshot,
        change,
        super::shortcut_copy::clipboard_change_count()?,
        || clipboard_win::empty().map_err(|e| e.to_string()),
        |format, data| {
            clipboard_win::raw::set_without_clear(format, data).map_err(|e| e.to_string())
        },
    )
}

#[cfg(target_os = "windows")]
fn open_clipboard() -> Result<clipboard_win::Clipboard, String> {
    let owner = unsafe { windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow() };
    clipboard_win::Clipboard::new_for(owner.0 as _).map_err(|error| error.to_string())
}

#[cfg(not(target_os = "windows"))]
pub(super) fn snapshot() -> Result<ClipboardSnapshot, String> {
    Err("Windows clipboard is unavailable on this platform".into())
}

#[cfg(not(target_os = "windows"))]
pub(super) fn restore(_: &ClipboardSnapshot, _: &ClipboardTextChange) -> Result<(), String> {
    Err("Windows clipboard is unavailable on this platform".into())
}

fn restore_with(
    snapshot: &ClipboardSnapshot,
    change: &ClipboardTextChange,
    current_change_count: i64,
    clear: impl FnOnce() -> Result<(), String>,
    mut set: impl FnMut(u32, &[u8]) -> Result<(), String>,
) -> Result<(), String> {
    if snapshot.change_count != change.before_change_count
        || current_change_count != change.change_count
    {
        return Ok(());
    }
    clear()?;
    let mut failure = None;
    for (format, data) in &snapshot.formats {
        if let Err(error) = set(*format, data) {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change() -> ClipboardTextChange {
        ClipboardTextChange {
            text: "selection".into(),
            before_change_count: 4,
            change_count: 5,
        }
    }

    #[test]
    fn restores_common_formats_together_after_clearing_once() {
        for formats in [
            vec![13],
            vec![13, 49152],
            vec![8, 17, 49153],
            vec![15, 49154],
            vec![],
        ] {
            let snapshot = ClipboardSnapshot {
                change_count: 4,
                formats: formats
                    .into_iter()
                    .map(|format| (format, vec![1, 2, 0]))
                    .collect(),
            };
            let mut clears = 0;
            let mut writes = Vec::new();
            restore_with(
                &snapshot,
                &change(),
                5,
                || {
                    clears += 1;
                    Ok(())
                },
                |format, data| {
                    writes.push((format, data.to_vec()));
                    Ok(())
                },
            )
            .unwrap();
            assert_eq!(clears, 1);
            assert_eq!(writes, snapshot.formats);
        }
    }

    #[test]
    fn never_overwrites_a_copy_before_or_after_the_temporary_copy() {
        for (snapshot_revision, current_revision) in [(3, 5), (4, 6)] {
            let snapshot = ClipboardSnapshot {
                change_count: snapshot_revision,
                formats: vec![],
            };
            restore_with(
                &snapshot,
                &change(),
                current_revision,
                || panic!("must not clear"),
                |_, _| panic!("must not write"),
            )
            .unwrap();
        }
    }

    #[test]
    fn reports_write_failures_and_still_attempts_remaining_formats() {
        let snapshot = ClipboardSnapshot {
            change_count: 4,
            formats: vec![(13, vec![0]), (15, vec![1])],
        };
        let mut writes = Vec::new();
        let result = restore_with(
            &snapshot,
            &change(),
            5,
            || Ok(()),
            |format, _| {
                writes.push(format);
                if format == 13 {
                    Err("restore failed".into())
                } else {
                    Ok(())
                }
            },
        );
        assert_eq!(result, Err("restore failed".into()));
        assert_eq!(writes, vec![13, 15]);
        assert!(restore_with(
            &snapshot,
            &change(),
            5,
            || Err("clear failed".into()),
            |_, _| panic!("must not write")
        )
        .is_err());
    }
}
