use std::path::PathBuf;

#[cfg(target_os = "macos")]
pub(crate) async fn select_native_result_image_file() -> crate::Result<Option<PathBuf>> {
    let future = crate::infrastructure::ui_thread::run("select OCR image", || {
        Ok(rfd::AsyncFileDialog::new()
            .set_title("选择图片进行 OCR")
            .add_filter(
                "图片",
                &["png", "jpg", "jpeg", "webp", "bmp", "gif", "tiff"],
            )
            .set_can_create_directories(true)
            .pick_file())
    })
    .await?;
    Ok(future.await.map(|file| file.path().to_path_buf()))
}

#[cfg(target_os = "macos")]
pub(crate) async fn save_native_capture_file(
    default_path: PathBuf,
    format: &str,
) -> crate::Result<Option<PathBuf>> {
    let format = capture_format(format)?.to_owned();
    let mut dialog = rfd::AsyncFileDialog::new()
        .set_title("保存截图")
        .set_can_create_directories(true);
    if let Some(directory) = default_path.parent() {
        dialog = dialog.set_directory(directory);
    }
    if let Some(name) = default_path.file_name() {
        dialog = dialog.set_file_name(name.to_string_lossy());
    }
    for (name, extensions) in capture_filters(&format) {
        dialog = dialog.add_filter(name, extensions);
    }
    let future =
        crate::infrastructure::ui_thread::run("save capture image", move || Ok(dialog.save_file()))
            .await?;
    Ok(future
        .await
        .map(|file| complete_capture_extension(file.path().to_path_buf(), &format)))
}

#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
pub(crate) fn show_native_error(title: &str, message: &str) {
    let title = title.to_owned();
    let message = message.to_owned();
    crate::infrastructure::runtime::spawn(async move {
        let result = crate::infrastructure::ui_thread::run("show error dialog", move || {
            Ok(rfd::AsyncMessageDialog::new()
                .set_title(title)
                .set_description(message)
                .set_level(rfd::MessageLevel::Error)
                .set_buttons(rfd::MessageButtons::Ok)
                .show())
        })
        .await;
        match result {
            Ok(future) => {
                let _ = future.await;
            }
            Err(error) => log::warn!("Failed to show native error dialog: {error}"),
        }
    });
}

fn capture_format(format: &str) -> crate::Result<&str> {
    match format {
        "png" | "jpg" | "jpeg" | "webp" => Ok(format),
        _ => Err("Unsupported capture image format".into()),
    }
}

fn capture_filters(format: &str) -> [(&'static str, &'static [&'static str]); 3] {
    let mut filters = [
        ("PNG（无损）", &["png"][..]),
        ("JPEG", &["jpg", "jpeg"][..]),
        ("WebP（无损）", &["webp"][..]),
    ];
    filters.sort_by_key(|(_, extensions)| !extensions.contains(&format));
    filters
}

fn complete_capture_extension(mut path: PathBuf, format: &str) -> PathBuf {
    if path.extension().is_none() {
        path.set_extension(format);
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn puts_the_selected_save_format_first_and_keeps_other_choices() {
        for format in ["png", "jpg", "jpeg", "webp"] {
            let filters = capture_filters(format);
            assert!(filters[0].1.contains(&format));
            assert_eq!(
                filters
                    .iter()
                    .map(|(_, extensions)| extensions.len())
                    .sum::<usize>(),
                4
            );
        }
        assert!(capture_format("../../executable").is_err());
    }

    #[test]
    fn appends_a_default_suffix_without_replacing_the_user_selected_one() {
        let root = Path::new("截图 目录");
        assert_eq!(
            complete_capture_extension(root.join("选区"), "webp"),
            root.join("选区.webp")
        );
        assert_eq!(
            complete_capture_extension(root.join("选区.JPEG"), "png"),
            root.join("选区.JPEG")
        );
        assert_eq!(
            complete_capture_extension(Path::new("目录.with.dot").join("选区"), "png"),
            Path::new("目录.with.dot").join("选区.png")
        );
    }
}
