use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub(crate) async fn select_ocr_image_file(app: AppHandle) -> Result<Option<String>, String> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    // A shortcut can start file OCR while the result window is still hidden.
    // Keep the picker independent so AppKit does not expose that window's old page.
    app.dialog()
        .file()
        .set_title("上传图片 OCR")
        .add_filter(
            "Images",
            &["png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff"],
        )
        .pick_file(move |selected| {
            let _ = sender.send(selected);
        });
    receiver
        .await
        .map_err(|error| error.to_string())?
        .map(|selected| {
            selected
                .into_path()
                .map(|path| path.to_string_lossy().into_owned())
                .map_err(|error| error.to_string())
        })
        .transpose()
}
