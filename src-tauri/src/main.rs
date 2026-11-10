#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_icon;
mod hotkey;
mod media_controls;
mod settings;
mod site;
mod tray;
mod updates;
mod windows_host;

use serde::{Deserialize, Serialize};
use settings::{Preferences, Settings};
use site::{CONTENT, SHELL};
use std::sync::Mutex;
use tauri::{
    webview::{NewWindowResponse, PageLoadEvent, WebviewBuilder},
    window::WindowBuilder,
    AppHandle, LogicalPosition, LogicalSize, Manager, Rect, State, Webview, WebviewUrl, Window,
};

#[derive(Clone, Copy, Deserialize)]
struct Pane {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Default)]
struct PaneState {
    pane: Option<Pane>,
    minimized: bool,
    hidden: bool,
    fullscreen: bool,
    maintenance: bool,
}

type ViewState = Mutex<PaneState>;

fn error(err: impl std::fmt::Display) -> String {
    err.to_string()
}

fn check_shell(view: &Webview) -> Result<(), String> {
    if view.label() == SHELL && view.url().is_ok_and(|url| is_shell_url(&url)) {
        Ok(())
    } else {
        Err("Only the local shell can run this command.".into())
    }
}

fn is_shell_url(url: &tauri::Url) -> bool {
    url.scheme() == "http"
        && url.host_str() == Some("tauri.localhost")
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
}

fn apply_bounds(view: &Webview, pane: Pane) -> tauri::Result<()> {
    view.set_bounds(Rect {
        position: LogicalPosition::new(pane.x, pane.y).into(),
        size: LogicalSize::new(pane.width, pane.height).into(),
    })
}

fn sync_media(app: &AppHandle) {
    let state = app.state::<ViewState>();
    let state = state.lock().expect("view state");
    let settings = app.state::<Preferences>();
    let auto_pause = settings
        .current
        .lock()
        .expect("preferences")
        .pause_minimized;
    if let Ok(view) = site::content(app) {
        let suspend = state.pane.is_none()
            || state.hidden
            || state.maintenance
            || (state.minimized && auto_pause);
        let script = if suspend {
            "window.dispatchEvent(new Event('desktop-media-suspend'))"
        } else {
            "window.dispatchEvent(new Event('desktop-media-restore'))"
        };
        let _ = view.eval(script);
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsSnapshot {
    #[serde(flatten)]
    settings: Settings,
    shortcut_active: bool,
}

#[tauri::command]
fn get_settings(
    webview: Webview,
    preferences: State<Preferences>,
) -> Result<SettingsSnapshot, String> {
    check_shell(&webview)?;
    Ok(SettingsSnapshot {
        settings: *preferences.current.lock().map_err(error)?,
        shortcut_active: webview.app_handle().state::<hotkey::Hotkey>().is_active(),
    })
}

#[tauri::command]
fn set_window_shortcut(webview: Webview, shortcut: Option<hotkey::Shortcut>) -> Result<(), String> {
    check_shell(&webview)?;
    webview
        .app_handle()
        .state::<hotkey::Hotkey>()
        .change(&webview.window(), shortcut)
}

#[tauri::command]
fn save_settings(
    webview: Webview,
    preferences: State<Preferences>,
    mut settings: Settings,
) -> Result<bool, String> {
    check_shell(&webview)?;
    if !(50..=175).contains(&settings.display.zoom_percent) {
        return Err("Page zoom must be between 50% and 175%.".into());
    }
    {
        let mut current = preferences.current.lock().map_err(error)?;
        settings.always_on_top = current.always_on_top;
        settings.hide_from_taskbar = current.hide_from_taskbar;
        settings.window_shortcut = current.window_shortcut;
        settings::persist(&preferences.file, &settings).map_err(error)?;
        *current = settings;
    }
    let app = webview.app_handle();
    if let Ok(view) = site::content(app) {
        windows_host::apply_theme(&view, settings.theme);
        view.set_zoom(settings.display.zoom_factor())
            .map_err(error)?;
        view.eval(site::display_script(settings.display))
            .map_err(error)?;
    }
    sync_media(app);
    Ok(preferences.startup_gpu != settings.hardware_acceleration)
}

#[tauri::command]
fn layout(webview: Webview, pane: Option<Pane>, focus: bool) -> Result<(), String> {
    check_shell(&webview)?;
    if let Some(rect) = pane {
        if ![rect.x, rect.y, rect.width, rect.height]
            .iter()
            .all(|v| (0.0..=32768.0).contains(v))
            || rect.width < 1.0
            || rect.height < 1.0
        {
            return Err("Invalid webview bounds.".into());
        }
    }
    let app = webview.app_handle();
    {
        let state = app.state::<ViewState>();
        let mut state = state.lock().map_err(error)?;
        state.pane = pane;
        let Ok(view) = site::content(app) else {
            return Ok(());
        };
        if let Some(pane) = pane {
            if !state.fullscreen {
                apply_bounds(&view, pane).map_err(error)?;
            }
            view.show().map_err(error)?;
            if focus {
                view.set_focus().map_err(error)?;
            }
        } else {
            view.hide().map_err(error)?;
            if focus {
                webview.set_focus().map_err(error)?;
            }
        }
    }
    sync_media(app);
    Ok(())
}

#[tauri::command]
fn open_qr_sign_in(webview: Webview) -> Result<(), String> {
    check_shell(&webview)?;
    let app = webview.app_handle();
    if app.state::<ViewState>().lock().map_err(error)?.maintenance {
        return Err("Wait for the browser data operation to finish.".into());
    }
    site::content(app)?
        .navigate(site::url("/login/qrcode"))
        .map_err(error)
}

#[tauri::command]
async fn clear_tiktok_data(webview: Webview, sign_out: bool) -> Result<(), String> {
    check_shell(&webview)?;
    let app = webview.app_handle();
    let view = site::content(app)?;
    let state = app.state::<ViewState>();
    {
        let mut state = state.lock().map_err(error)?;
        if state.maintenance {
            return Err("A browser data operation is already running.".into());
        }
        state.maintenance = true;
    }
    sync_media(app);
    let result = windows_host::clear_data(&view, sign_out).await;
    if sign_out {
        let _ = view.navigate(site::url("/login"));
    } else {
        let _ = view.reload();
    }
    state.lock().map_err(error)?.maintenance = false;
    sync_media(app);
    result
}

fn show_window(window: &Window) {
    if window.show().is_err() {
        return;
    }
    let _ = window.unminimize();
    let app = window.app_handle();
    let hide_from_taskbar = app
        .state::<Preferences>()
        .current
        .lock()
        .expect("preferences")
        .hide_from_taskbar;
    if hide_from_taskbar {
        let _ = window.set_skip_taskbar(true);
    }
    app.state::<ViewState>().lock().expect("view state").hidden = false;
    sync_media(app);
    let _ = window.set_focus();
}

fn toggle_window(window: &Window) {
    if !window.is_visible().unwrap_or(false) || window.is_minimized().unwrap_or(false) {
        show_window(window);
    } else if window.hide().is_ok() {
        let app = window.app_handle();
        app.state::<ViewState>().lock().expect("view state").hidden = true;
        sync_media(app);
    }
}

fn on_minimized(window: &Window, minimized: bool) {
    let app = window.app_handle();
    let state = app.state::<ViewState>();
    let mut state = state.lock().expect("view state");
    if state.minimized != minimized {
        state.minimized = minimized;
        drop(state);
        sync_media(app);
    }
}

fn on_fullscreen(window: &Window, fullscreen: bool) {
    let app = window.app_handle();
    let state = app.state::<ViewState>();
    let pane = {
        let mut state = state.lock().expect("view state");
        if fullscreen && (state.pane.is_none() || state.maintenance) {
            return;
        }
        state.fullscreen = fullscreen;
        state.pane
    };
    let _ = window.set_fullscreen(fullscreen);
    if !fullscreen {
        if let (Some(pane), Ok(view)) = (pane, site::content(app)) {
            let _ = apply_bounds(&view, pane);
        }
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(updates::UpdateState::default())
        .manage(hotkey::Hotkey::default())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_window_shortcut,
            save_settings,
            layout,
            open_qr_sign_in,
            clear_tiktok_data
        ])
        .setup(|app| {
            let data = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&data)?;
            let preferences = Preferences::load(data.join("settings.json"));
            let initial = *preferences.current.lock().expect("preferences");
            app.manage(preferences);

            let pane = Pane { x: 0.0, y: 0.0, width: 1120.0, height: 760.0 };
            app.manage(Mutex::new(PaneState { pane: Some(pane), ..PaneState::default() }));
            let icon_path = data.join("app-icon.ico");
            let custom_icon = app_icon::load(&icon_path);
            let mut window = WindowBuilder::new(app, "desktop")
                .title("TikTok Desktop")
                .inner_size(1120.0, 760.0)
                .min_inner_size(280.0, 260.0)
                .decorations(true)
                .always_on_top(initial.always_on_top)
                .skip_taskbar(initial.hide_from_taskbar)
                .visible(false);
            let size = app_icon::system_size(windows::Win32::UI::WindowsAndMessaging::SM_CXSMICON);
            if let Some(icon) = custom_icon.as_ref().and_then(|icon| app_icon::frame(icon, size)) {
                window = window.icon(icon)?;
            }
            let window = window.build()?;
            if custom_icon.is_some() {
                if let Some(icon) = app_icon::set_taskbar(&window, &icon_path) {
                    app.manage(icon);
                }
            }

            let profile = data.join("TikTokProfile");
            let args = if initial.hardware_acceleration {
                "--disable-features=msWebOOUI,msPdfOOUI,HardwareMediaKeyHandling --autoplay-policy=no-user-gesture-required"
            } else {
                "--disable-features=msWebOOUI,msPdfOOUI,HardwareMediaKeyHandling --autoplay-policy=no-user-gesture-required --disable-gpu"
            };
            window.add_child(
                WebviewBuilder::new(SHELL, WebviewUrl::App("index.html".into()))
                    .on_navigation(is_shell_url)
                    .data_directory(profile.clone())
                    .additional_browser_args(args)
                    .incognito(true)
                    .on_new_window(|_, _| NewWindowResponse::Deny)
                    .disable_drag_drop_handler()
                    .devtools(false)
                    .zoom_hotkeys_enabled(false)
                    .auto_resize(),
                LogicalPosition::new(0.0, 0.0),
                window.inner_size()?,
            )?;

            let handle = app.handle().clone();
            let builder = WebviewBuilder::new(CONTENT, WebviewUrl::External("about:blank".parse()?))
                .on_navigation(|url| url.as_str() == "about:blank" || site::is_tiktok(url))
                .on_new_window(move |url, _| {
                    if site::is_tiktok(&url) {
                        let app = handle.clone();
                        let _ = handle.run_on_main_thread(move || {
                            if let Ok(view) = site::content(&app) { let _ = view.navigate(url); }
                        });
                    } else if let Some(shell) = handle.get_webview(SHELL) {
                        let _ = shell.eval("window.dispatchEvent(new Event('desktop-popup-blocked'))");
                    }
                    NewWindowResponse::Deny
                })
                .on_page_load(|view, payload| {
                    if payload.event() == PageLoadEvent::Finished && site::is_tiktok(payload.url()) {
                        let preferences = view.app_handle().state::<Preferences>();
                        let display = preferences.current.lock().expect("preferences").display;
                        let _ = view.eval(site::display_script(display));
                        sync_media(view.app_handle());
                    }
                })
                .initialization_script(site::LIFECYCLE)
                .initialization_script(site::display_script(initial.display))
                .data_directory(profile)
                .additional_browser_args(args)
                .disable_drag_drop_handler()
                .general_autofill_enabled(false)
                .on_download(|_, _| false)
                .on_permission_request(|_, _| tauri::webview::PermissionResponse::Deny)
                .devtools(false)
                .focused(false);

            let view = window.add_child(
                builder,
                LogicalPosition::new(pane.x, pane.y),
                LogicalSize::new(pane.width, pane.height),
            )?;

            let requested_pane = app.state::<ViewState>().lock().expect("view state").pane;
            if let Some(requested_pane) = requested_pane {
                apply_bounds(&view, requested_pane)?;
            } else {
                view.hide()?;
            }
            windows_host::configure(&window, &view, initial)?;
            if let Some(shortcut) = initial.window_shortcut {
                if let Err(err) = app.state::<hotkey::Hotkey>().start(&window, shortcut) {
                    eprintln!("Window shortcut unavailable: {err}");
                }
            }
            window.on_window_event({
                let window = window.clone();
                move |event| {
                    if let tauri::WindowEvent::Resized(size) = event {
                        if window.app_handle().state::<ViewState>().lock().expect("view state").fullscreen {
                            if let Ok(view) = site::content(window.app_handle()) {
                                let _ = view.set_bounds(Rect {
                                    position: LogicalPosition::new(0.0, 0.0).into(),
                                    size: (*size).into(),
                                });
                            }
                        }
                    }
                }
            });
            tray::install(app, custom_icon.as_ref(), initial)?;
            window.show()?;
            if initial.hide_from_taskbar {
                window.set_skip_taskbar(true)?;
            }
            view.set_focus()?;
            updates::check(app.handle(), false);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("TikTok Desktop failed");
}
