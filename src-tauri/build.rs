fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "get_settings",
            "set_window_shortcut",
            "save_settings",
            "layout",
            "open_qr_sign_in",
            "clear_tiktok_data",
        ]),
    ))
    .expect("failed to generate Tauri build metadata");
}
