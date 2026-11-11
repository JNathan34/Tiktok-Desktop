use crate::{
    app_icon, error, is_shell_url,
    settings::{self, Preferences, Theme},
    settings_window, site, updates,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{
    image::Image,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    webview::{NewWindowResponse, WebviewWindowBuilder},
    App, AppHandle, Manager, PhysicalPosition, PhysicalSize, Rect, Webview, WebviewUrl, Window,
    WindowEvent,
};
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::{
            PostMessageW, SM_CXSMICON, WA_INACTIVE, WM_ACTIVATE, WM_CLOSE, WM_NCDESTROY,
        },
    },
};

const LABEL: &str = "tray-popup";
const WIDTH: u32 = 312;
const HEIGHT: u32 = 376;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    VideoOnly,
    Settings,
    AlwaysOnTop,
    HideFromTaskbar,
    Reload,
    Updates,
    Quit,
    Dismiss,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuState {
    theme: Theme,
    video_only: bool,
    always_on_top: bool,
    hide_from_taskbar: bool,
}

fn is_tray_url(url: &tauri::Url) -> bool {
    is_shell_url(url) && url.path() == "/tray.html"
}

fn tray_app(view: &Webview) -> Result<&AppHandle, String> {
    if view.label() == LABEL && view.url().is_ok_and(|url| is_tray_url(&url)) {
        Ok(view.app_handle())
    } else {
        Err("Only the local tray menu can run this command.".into())
    }
}

fn menu_state(app: &AppHandle) -> Result<MenuState, String> {
    let preferences = app.state::<Preferences>();
    let current = preferences.current.lock().map_err(error)?;
    Ok(MenuState {
        theme: current.theme,
        video_only: current.display.video_only,
        always_on_top: current.always_on_top,
        hide_from_taskbar: current.hide_from_taskbar,
    })
}

#[tauri::command]
pub fn tray_state(webview: Webview) -> Result<MenuState, String> {
    menu_state(tray_app(&webview)?)
}

fn toggle_window_option(window: &Window, action: Action) -> Result<(), String> {
    let preferences = window.app_handle().state::<Preferences>();
    let mut current = preferences.current.lock().map_err(error)?;
    let mut next = *current;
    let (value, apply): (_, fn(&Window, bool) -> tauri::Result<()>) = match action {
        Action::AlwaysOnTop => (&mut next.always_on_top, Window::set_always_on_top),
        Action::HideFromTaskbar => (&mut next.hide_from_taskbar, Window::set_skip_taskbar),
        _ => unreachable!(),
    };
    *value = !*value;
    let checked = *value;
    apply(window, checked).map_err(error)?;
    if let Err(err) = settings::persist(&preferences.file, &next) {
        let _ = apply(window, !checked);
        return Err(error(err));
    }
    *current = next;
    Ok(())
}

#[tauri::command]
pub fn tray_action(webview: Webview, action: Action) -> Result<MenuState, String> {
    let app = tray_app(&webview)?;
    let popup = webview.window();
    match action {
        Action::Dismiss => popup.hide().map_err(error)?,
        Action::Quit => app.exit(0),
        Action::VideoOnly => {
            let preferences = app.state::<Preferences>();
            let mut current = preferences.current.lock().map_err(error)?;
            let mut next = *current;
            next.display.video_only = !next.display.video_only;
            settings::persist(&preferences.file, &next).map_err(error)?;
            *current = next;
            if let Some(settings) = app.get_webview(settings_window::LABEL) {
                let _ = settings.eval("window.dispatchEvent(new Event('desktop-video-only-changed'))");
            }
            if let Ok(view) = site::content(app) {
                view.eval(site::display_script(next.display)).map_err(error)?;
            }
        }
        _ => {
            let window = app
                .get_window("desktop")
                .ok_or("TikTok window is unavailable.")?;
            match action {
                Action::AlwaysOnTop | Action::HideFromTaskbar => {
                    toggle_window_option(&window, action)?
                }
                Action::Settings => {
                    popup.hide().map_err(error)?;
                    settings_window::show(app)?;
                }
                Action::Updates => {
                    popup.hide().map_err(error)?;
                    crate::show_window(&window);
                    updates::check(app, true);
                }
                Action::Reload => {
                    crate::reload_feed(app)?;
                    popup.hide().map_err(error)?;
                    crate::show_window(&window);
                    if let Some(shell) = app.get_webview(site::SHELL) {
                        let _ = shell.eval("window.dispatchEvent(new Event('desktop-feed-reloaded'))");
                    }
                }
                _ => unreachable!(),
            }
        }
    }
    menu_state(app)
}

fn show_popup(app: &AppHandle, anchor: Rect) -> tauri::Result<()> {
    let Some(popup) = app.get_webview_window(LABEL) else {
        return Ok(());
    };
    let icon = anchor.position.to_physical::<i32>(1.0);
    let icon_size = anchor.size.to_physical::<i32>(1.0);
    let monitor = popup
        .monitor_from_point(f64::from(icon.x), f64::from(icon.y))?
        .or(popup.primary_monitor()?);
    if let Some(monitor) = monitor {
        let scale = monitor.scale_factor();
        let work = monitor.work_area();
        let gap = (8.0 * scale).round() as i32;
        let width = ((f64::from(WIDTH) * scale).round() as u32).min(work.size.width);
        let height = ((f64::from(HEIGHT) * scale).round() as u32).min(work.size.height);
        let x = (icon.x + (icon_size.width - width as i32) / 2).clamp(
            work.position.x,
            work.position.x + (work.size.width - width) as i32,
        );
        let above = icon.y - height as i32 - gap;
        let y = if above >= work.position.y {
            above
        } else {
            icon.y + icon_size.height + gap
        };
        let y = y.clamp(
            work.position.y,
            work.position.y + (work.size.height - height) as i32,
        );
        popup.set_position(PhysicalPosition::new(x, y))?;
        popup.set_size(PhysicalSize::new(width, height))?;
    }
    popup.show()?;
    popup.set_focus()?;
    popup.as_ref().set_focus()?;
    popup.eval("window.dispatchEvent(new Event('tray-opening'))")
}

unsafe extern "system" fn observe_popup(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    id: usize,
    _: usize,
) -> LRESULT {
    unsafe {
        // Use native window activation. WebView focus can change within the popup.
        if message == WM_ACTIVATE && (wparam.0 & 0xffff) == WA_INACTIVE as usize {
            let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
        } else if message == WM_NCDESTROY {
            let _ = RemoveWindowSubclass(hwnd, Some(observe_popup), id);
        }
        DefSubclassProc(hwnd, message, wparam, lparam)
    }
}

pub fn install(
    app: &App,
    custom_icon: Option<&ico::IconDir>,
    profile: PathBuf,
    browser_args: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let icon = |size| match custom_icon.and_then(|icon| app_icon::frame(icon, size)) {
        Some(image) => Ok(image),
        None => Image::from_app_icon_resource(size),
    };
    let brand = icon(96)?;
    let data = serde_json::json!({
        "version": app.package_info().version.to_string(),
        "width": brand.width(),
        "height": brand.height(),
        "pixels": brand.rgba(),
    });
    let popup = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("tray.html".into()))
        .title("TikTok Desktop tray menu")
        .inner_size(f64::from(WIDTH), f64::from(HEIGHT))
        .visible(false)
        .focused(false)
        .decorations(false)
        .resizable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .data_directory(profile)
        .additional_browser_args(browser_args)
        .incognito(true)
        .disable_drag_drop_handler()
        .zoom_hotkeys_enabled(false)
        .devtools(false)
        .on_navigation(is_tray_url)
        .on_new_window(|_, _| NewWindowResponse::Deny)
        .initialization_script(format!("window.trayBrand = {data};"))
        .build()?;
    if !unsafe { SetWindowSubclass(popup.hwnd()?, Some(observe_popup), 1, 0) }.as_bool() {
        return Err("Could not observe tray menu activation.".into());
    }
    let window = popup.clone();
    popup.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = window.hide();
        }
    });

    TrayIconBuilder::with_id("tiktok-desktop")
        .icon(icon(app_icon::system_size(SM_CXSMICON))?)
        .tooltip("TikTok Desktop · Left-click to open · Right-click for controls")
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                let app = tray.app_handle();
                match button {
                    MouseButton::Left => {
                        if let Some(popup) = app.get_webview_window(LABEL) {
                            let _ = popup.hide();
                        }
                        if let Some(window) = app.get_window("desktop") {
                            crate::show_window(&window);
                        }
                    }
                    MouseButton::Right => {
                        if let Err(err) = show_popup(app, rect) {
                            eprintln!("Could not open tray menu: {err}");
                        }
                    }
                    _ => {}
                }
            }
        })
        .build(app)?;
    Ok(())
}
