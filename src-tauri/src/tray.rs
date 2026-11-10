use crate::{
    app_icon, error,
    settings::{self, Preferences, Settings},
    site, updates,
};
use tauri::{
    image::Image,
    menu::{CheckMenuItem, IconMenuItem, Menu, PredefinedMenuItem},
    tray::TrayIconBuilder,
    App, AppHandle, Manager,
};
use windows::Win32::UI::WindowsAndMessaging::SM_CXSMICON;

fn toggle_always_on_top(app: &AppHandle, item: &CheckMenuItem<tauri::Wry>) -> Result<(), String> {
    let window = app
        .get_window("desktop")
        .ok_or("TikTok window is unavailable.")?;
    let preferences = app.state::<Preferences>();
    let mut current = preferences.current.lock().map_err(error)?;
    let next = Settings {
        always_on_top: !current.always_on_top,
        ..*current
    };
    window
        .set_always_on_top(next.always_on_top)
        .map_err(error)?;
    if let Err(err) = settings::persist(&preferences.file, &next) {
        let _ = window.set_always_on_top(current.always_on_top);
        return Err(error(err));
    }
    *current = next;
    item.set_checked(next.always_on_top).map_err(error)
}

fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_window("desktop") {
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn install(
    app: &App,
    custom_icon: Option<&ico::IconDir>,
    topmost: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let size = app_icon::system_size(SM_CXSMICON);
    let icon = match custom_icon.and_then(|icon| app_icon::frame(icon, size)) {
        Some(icon) => icon,
        None => Image::from_app_icon_resource(size)?,
    };
    let header = IconMenuItem::new(
        app,
        format!("TikTok Desktop  ·  v{}", app.package_info().version),
        false,
        Some(icon.clone()),
        None::<&str>,
    )?;
    let action = |id, label, mask, colour| {
        IconMenuItem::with_id(
            app,
            id,
            label,
            true,
            Some(glyph(size, mask, colour)),
            None::<&str>,
        )
    };
    let open = action("tray-open", "Open TikTok", OPEN, [43, 117, 168])?;
    let settings = action("tray-settings", "Settings", SETTINGS, [105, 101, 168])?;
    let check_updates = action("tray-updates", "Check for updates", UPDATE, [31, 125, 107])?;
    let quit = action("tray-quit", "Quit TikTok Desktop", QUIT, [164, 69, 92])?;
    let always_on_top = CheckMenuItem::with_id(
        app,
        "tray-topmost",
        "Always on top",
        true,
        topmost,
        None::<&str>,
    )?;
    let header_rule = PredefinedMenuItem::separator(app)?;
    let window_rule = PredefinedMenuItem::separator(app)?;
    let app_rule = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[
            &header,
            &header_rule,
            &open,
            &settings,
            &window_rule,
            &always_on_top,
            &app_rule,
            &check_updates,
            &quit,
        ],
    )?;

    TrayIconBuilder::with_id("tiktok-desktop")
        .icon(icon)
        .tooltip("TikTok Desktop")
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "tray-open" => show_window(app),
            "tray-settings" => {
                show_window(app);
                if let Some(shell) = app.get_webview(site::SHELL) {
                    let _ = shell.eval("window.dispatchEvent(new Event('desktop-settings'))");
                }
            }
            "tray-topmost" => {
                if let Err(err) = toggle_always_on_top(app, &always_on_top) {
                    eprintln!("Could not change Always on top: {err}");
                }
            }
            "tray-updates" => {
                show_window(app);
                updates::check(app, true);
            }
            "tray-quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

// Scale the monochrome menu artwork to the system menu icon size.
fn glyph(size: u32, mask: &[&str; 12], [r, g, b]: [u8; 3]) -> Image<'static> {
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        let row = mask[(y * 12 / size) as usize].as_bytes();
        for x in 0..size {
            let alpha = if row[(x * 12 / size) as usize] == b'#' {
                255
            } else {
                0
            };
            pixels.extend([r, g, b, alpha]);
        }
    }
    Image::new_owned(pixels, size, size)
}

const OPEN: &[&str; 12] = &[
    "............",
    ".##########.",
    ".#........#.",
    ".#..#.....#.",
    ".#..###...#.",
    ".#..####..#.",
    ".#..###...#.",
    ".#..#.....#.",
    ".#........#.",
    ".##########.",
    "............",
    "............",
];

const SETTINGS: &[&str; 12] = &[
    "............",
    "............",
    "..#...#...#.",
    "..#...#...#.",
    ".###..#...#.",
    "..#..###..#.",
    "..#...#..###",
    "..#...#...#.",
    "..#...#...#.",
    "............",
    "............",
    "............",
];

const UPDATE: &[&str; 12] = &[
    "............",
    ".....##.....",
    ".....##.....",
    ".....##.....",
    "...#.##.#...",
    "....####....",
    ".....##.....",
    "............",
    ".#........#.",
    ".##########.",
    "............",
    "............",
];

const QUIT: &[&str; 12] = &[
    "............",
    ".######.....",
    ".#..........",
    ".#......#...",
    ".#.......#..",
    ".#...######.",
    ".#.......#..",
    ".#......#...",
    ".#..........",
    ".######.....",
    "............",
    "............",
];
