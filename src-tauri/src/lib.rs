// Module declarations
mod commands;
mod config;
mod language;
mod ocr;
mod translate;
mod capture;
mod history;
mod utils;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_shell::init())
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      // TODO: Initialize configuration
      // TODO: Register global hotkeys
      // TODO: Create system tray

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      commands::capture_screen,
      commands::get_config,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
