use crate::{error, is_shell_url};
use std::path::PathBuf;
use tauri::{
    webview::{NewWindowResponse, WebviewWindowBuilder},
    App, AppHandle, Manager, Webview, WebviewUrl, Window, WindowEvent,
};

pub const LABEL: &str = "settings";

fn is_settings_url(url: &tauri::Url) -> bool {
    is_shell_url(url) && url.path() == "/settings.html"
}

pub fn check(view: &Webview) -> Result<(), String> {
    if view.label() == LABEL && view.url().is_ok_and(|url| is_settings_url(&url)) {
        Ok(())
    } else {
        Err("Only the local Settings window can run this command.".into())
    }
}

pub fn install(app: &App, desktop: &Window, profile: PathBuf, args: &str) -> tauri::Result<()> {
    let settings = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("settings.html".into()))
        .title("TikTok Desktop Settings")
        .owner_raw(desktop.hwnd()?)
        .inner_size(690.0, 660.0)
        .min_inner_size(340.0, 360.0)
        .visible(false)
        .focused(false)
        .data_directory(profile)
        .additional_browser_args(args)
        .incognito(true)
        .disable_drag_drop_handler()
        .zoom_hotkeys_enabled(false)
        .devtools(false)
        .on_navigation(is_settings_url)
        .on_new_window(|_, _| NewWindowResponse::Deny)
        .build()?;
    let window = settings.clone();
    settings.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = window.hide();
        }
    });
    Ok(())
}

pub fn show(app: &AppHandle) -> Result<(), String> {
    let settings = app
        .get_webview_window(LABEL)
        .ok_or("Settings window is unavailable.")?;
    if let Some(desktop) = app.get_window("desktop") {
        if !desktop.is_visible().map_err(error)? || desktop.is_minimized().map_err(error)? {
            crate::show_window(&desktop);
        }
    }
    settings.show().map_err(error)?;
    settings.set_focus().map_err(error)?;
    settings.as_ref().set_focus().map_err(error)
}

#[tauri::command]
pub fn close_settings(webview: Webview) -> Result<(), String> {
    check(&webview)?;
    webview.window().hide().map_err(error)
}
