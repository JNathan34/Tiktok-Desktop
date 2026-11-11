use crate::{
    hotkey,
    settings::{Settings, Theme},
    site,
};
use std::{cell::Cell, rc::Rc, sync::mpsc, time::Duration};
use tauri::{Manager, Webview, Window};
use webview2_com::{
    AcceleratorKeyPressedEventHandler, ClearBrowsingDataCompletedHandler, CoTaskMemPWSTR,
    ContainsFullScreenElementChangedEventHandler, Microsoft::Web::WebView2::Win32::*,
    ExecuteScriptCompletedHandler, NavigationCompletedEventHandler,
};
use windows::{
    core::{Interface, HSTRING},
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        UI::{
            Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_MENU, VK_OEM_COMMA, VK_SHIFT},
            Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
            WindowsAndMessaging::{SIZE_MINIMIZED, WM_EXITSIZEMOVE, WM_HOTKEY, WM_NCDESTROY, WM_SIZE},
        },
    },
};

const SUBCLASS: usize = 1;

unsafe extern "system" fn observe_window(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    id: usize,
    data: usize,
) -> LRESULT {
    // The pointer belongs to this subclass and lives until WM_NCDESTROY on this UI thread.
    if msg == WM_NCDESTROY {
        hotkey::release(hwnd);
        unsafe {
            let _ = RemoveWindowSubclass(hwnd, Some(observe_window), id);
            drop(Box::from_raw(data as *mut Window));
        }
    } else if msg == WM_HOTKEY {
        let window = unsafe { &*(data as *const Window) };
        window
            .app_handle()
            .state::<hotkey::Hotkey>()
            .handle(window, wparam.0);
    } else if msg == WM_SIZE {
        let window = unsafe { &*(data as *const Window) };
        crate::on_minimized(window, wparam.0 == SIZE_MINIMIZED as usize);
    } else if msg == WM_EXITSIZEMOVE {
        let window = unsafe { &*(data as *const Window) };
        crate::remember_window(window);
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

pub fn configure(
    window: &Window,
    view: &Webview,
    initial: Settings,
) -> Result<(), Box<dyn std::error::Error>> {
    let hwnd = window.hwnd()?;
    let pointer = Box::into_raw(Box::new(window.clone()));
    if !unsafe { SetWindowSubclass(hwnd, Some(observe_window), SUBCLASS, pointer as usize) }
        .as_bool()
    {
        unsafe {
            drop(Box::from_raw(pointer));
        }
        return Err("Could not observe Windows minimize events.".into());
    }

    let native_window = hwnd.0 as usize;
    let window = window.clone();
    let view_handle = view.clone();
    view.with_webview(move |platform| {
        let setup = || -> windows::core::Result<()> {
            let controller = platform.controller();
            let core = unsafe { controller.CoreWebView2()? };
            // The site starts on a blank document. Disable WebView2 messaging before loading TikTok.
            unsafe { core.Settings()?.SetIsWebMessageEnabled(false)? };
            let _ = set_theme(&core, initial.theme);
            unsafe { controller.SetZoomFactor(initial.display.zoom_factor())? };
            if let Err(error) = crate::media_controls::attach(
                HWND(native_window as *mut _),
                &core,
                view_handle.clone(),
            ) {
                eprintln!("Could not register Windows media controls: {error}");
            }

            let app = window.app_handle().clone();
            let preferences =
                AcceleratorKeyPressedEventHandler::create(Box::new(move |_, args| {
                    let Some(args) = args else {
                        return Ok(());
                    };
                    let mut kind = COREWEBVIEW2_KEY_EVENT_KIND(0);
                    let mut key = 0;
                    unsafe {
                        args.KeyEventKind(&mut kind)?;
                        args.VirtualKey(&mut key)?;
                        if key == u32::from(VK_OEM_COMMA.0)
                            && kind == COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN
                            && GetKeyState(i32::from(VK_CONTROL.0)) < 0
                            && GetKeyState(i32::from(VK_MENU.0)) >= 0
                            && GetKeyState(i32::from(VK_SHIFT.0)) >= 0
                        {
                            args.SetHandled(true)?;
                            if let Err(err) = crate::settings_window::show(&app) {
                                eprintln!("Could not open Settings: {err}");
                            }
                        }
                    }
                    Ok(())
                }));
            let fullscreen =
                ContainsFullScreenElementChangedEventHandler::create(Box::new(move |sender, _| {
                    if let Some(core) = sender {
                        let mut active = Default::default();
                        unsafe { core.ContainsFullScreenElement(&mut active)? };
                        crate::on_fullscreen(&window, active.as_bool());
                    }
                    Ok(())
                }));
            unsafe {
                controller.add_AcceleratorKeyPressed(&preferences, &mut 0)?;
                core.add_ContainsFullScreenElementChanged(&fullscreen, &mut 0)?;
                view_handle.app_handle().state::<crate::ViewState>()
                    .lock().expect("view state").site_secured = true;
                core.Navigate(&HSTRING::from(site::feed_url(initial.launch_feed).as_str()))?;
            }
            Ok(())
        };
        match setup() {
            Ok(()) => {}
            Err(error) => {
                eprintln!("Could not secure the TikTok WebView2 controller: {error}");
                if let Some(shell) = view_handle.app_handle().get_webview(site::SHELL) {
                    let _ = shell.eval("window.dispatchEvent(new Event('desktop-load-error'))");
                }
            }
        }
    })?;
    Ok(())
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlaybackHealth {
    Blank,
    Buffering,
    Inactive,
    Ready,
    VideoError,
}

pub async fn playback_health(view: &Webview) -> Result<PlaybackHealth, String> {
    let (done, result) = mpsc::channel();
    view.with_webview(move |platform| {
        let complete = ExecuteScriptCompletedHandler::create(Box::new({
            let done = done.clone();
            move |status, json| {
                let _ = done.send(status.map_err(crate::error).and_then(|()| {
                    serde_json::from_str::<PlaybackHealth>(&json).map_err(crate::error)
                }));
                Ok(())
            }
        }));
        let script = HSTRING::from(include_str!("site_ready.js"));
        let query = unsafe {
            platform.controller().CoreWebView2()
                .and_then(|core| core.ExecuteScript(&script, &complete))
        };
        if let Err(error) = query {
            let _ = done.send(Err(error.to_string()));
        }
    })
    .map_err(crate::error)?;
    tauri::async_runtime::spawn_blocking(move || {
        result.recv_timeout(Duration::from_secs(3)).map_err(crate::error)?
    })
    .await
    .map_err(crate::error)?
}

fn set_theme(core: &ICoreWebView2, theme: Theme) -> windows::core::Result<()> {
    let scheme = match theme {
        Theme::Dark => COREWEBVIEW2_PREFERRED_COLOR_SCHEME_DARK,
        Theme::Light => COREWEBVIEW2_PREFERRED_COLOR_SCHEME_LIGHT,
        Theme::System => COREWEBVIEW2_PREFERRED_COLOR_SCHEME_AUTO,
    };
    unsafe {
        core.cast::<ICoreWebView2_13>()?
            .Profile()?
            .SetPreferredColorScheme(scheme)
    }
}

pub fn apply_theme(view: &Webview, theme: Theme) {
    let _ = view.with_webview(move |platform| {
        if let Ok(core) = unsafe { platform.controller().CoreWebView2() } {
            let _ = set_theme(&core, theme);
        }
    });
}

fn start_clear(core: &ICoreWebView2, sign_out: bool, done: mpsc::Sender<Result<(), String>>) {
    let complete = done.clone();
    let handler = ClearBrowsingDataCompletedHandler::create(Box::new(move |status| {
        let _ = complete.send(status.map_err(crate::error));
        Ok(())
    }));
    let result = (|| unsafe {
        let profile = core
            .cast::<ICoreWebView2_13>()?
            .Profile()?
            .cast::<ICoreWebView2Profile2>()?;
        if sign_out {
            profile.ClearBrowsingDataAll(&handler)
        } else {
            profile.ClearBrowsingData(COREWEBVIEW2_BROWSING_DATA_KINDS_DISK_CACHE, &handler)
        }
    })();
    if let Err(error) = result {
        let _ = done.send(Err(error.to_string()));
    }
}

fn clear_after_navigation(
    core: &ICoreWebView2,
    done: mpsc::Sender<Result<(), String>>,
) -> windows::core::Result<()> {
    // Discard the running page before deleting its storage, so it cannot write its old state back.
    let token = Rc::new(Cell::new(0));
    let callback_token = token.clone();
    let callback = NavigationCompletedEventHandler::create(Box::new(move |sender, _| {
        if let Some(core) = sender {
            unsafe { core.remove_NavigationCompleted(callback_token.get())? };
            let mut source = windows::core::PWSTR::null();
            unsafe { core.Source(&mut source)? };
            if CoTaskMemPWSTR::from(source).to_string() == "about:blank" {
                start_clear(&core, true, done.clone());
            } else {
                let _ = done.send(Err(
                    "Could not unload TikTok before clearing its session.".into()
                ));
            }
        }
        Ok(())
    }));
    unsafe {
        let mut id = 0;
        core.add_NavigationCompleted(&callback, &mut id)?;
        token.set(id);
        if let Err(error) = core.Navigate(&HSTRING::from("about:blank")) {
            let _ = core.remove_NavigationCompleted(id);
            return Err(error);
        }
    }
    Ok(())
}

pub async fn clear_data(view: &Webview, sign_out: bool) -> Result<(), String> {
    let (done, result) = mpsc::channel();
    let failed = done.clone();
    view.with_webview(move |platform| {
        let result = unsafe { platform.controller().CoreWebView2() }.and_then(|core| {
            if sign_out {
                clear_after_navigation(&core, done)
            } else {
                start_clear(&core, false, done);
                Ok(())
            }
        });
        if let Err(error) = result {
            let _ = failed.send(Err(error.to_string()));
        }
    })
    .map_err(crate::error)?;
    tauri::async_runtime::spawn_blocking(move || {
        result
            .recv_timeout(Duration::from_secs(20))
            .map_err(|_| "WebView2 did not finish clearing data.".to_owned())?
    })
    .await
    .map_err(crate::error)?
}
