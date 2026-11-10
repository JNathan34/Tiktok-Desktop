use crate::{
    error,
    settings::{self, Preferences},
    site,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::sync::{Mutex, OnceLock};
use tauri::{Manager, Window};
use windows::Win32::{
    Foundation::HWND,
    UI::Input::KeyboardAndMouse::{
        RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
        MOD_SHIFT, VIRTUAL_KEY, VK_DOWN, VK_LEFT, VK_RIGHT, VK_UP,
    },
};

const FIRST_ID: i32 = 0x5101;
const SECOND_ID: i32 = 0x5102;
const FIRST_PLAYBACK_ID: i32 = 0x5110;
const CTRL_SHIFT: u8 = (MOD_CONTROL.0 | MOD_SHIFT.0) as u8;
const PLAYBACK_KEYS: [(VIRTUAL_KEY, &str, &str); 4] = [
    (VK_UP, "Up", "desktop-media-previous"),
    (VK_DOWN, "Down", "desktop-media-next"),
    (VK_LEFT, "Left", "desktop-media-rewind"),
    (VK_RIGHT, "Right", "desktop-media-forward"),
];

#[derive(Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
pub struct Shortcut {
    #[serde(deserialize_with = "current_modifiers")]
    modifiers: u8,
    key: u8,
}

fn current_modifiers<'de, D: Deserializer<'de>>(input: D) -> Result<u8, D::Error> {
    const CTRL_ALT: u8 = (MOD_CONTROL.0 | MOD_ALT.0) as u8;
    // Carry over the selected key from saved Ctrl+Alt shortcuts.
    Ok(match u8::deserialize(input)? {
        CTRL_ALT => CTRL_SHIFT,
        modifiers => modifiers,
    })
}

impl Shortcut {
    fn register(self, hwnd: HWND, id: i32) -> Result<(), String> {
        let function_key = (0x70..=0x7a).contains(&self.key);
        let letter_or_digit = self.key.is_ascii_uppercase() || self.key.is_ascii_digit();
        if self.modifiers & !7 != 0
            || !(function_key || (letter_or_digit && self.modifiers & 3 != 0))
            || (self.key == 0x73 && self.modifiers == 1)
        {
            return Err("Choose F1–F11, or use Ctrl or Alt with a letter or number. Alt+F4 closes the window.".into());
        }
        let modifiers = HOT_KEY_MODIFIERS(u32::from(self.modifiers)) | MOD_NOREPEAT;
        unsafe { RegisterHotKey(Some(hwnd), id, modifiers, u32::from(self.key)) }.map_err(|_| {
            "Windows could not register this shortcut. It may already be in use.".into()
        })
    }
}

#[derive(Default)]
pub struct Hotkey {
    window: Mutex<Option<i32>>,
    unavailable_playback: OnceLock<Vec<&'static str>>,
}

impl Hotkey {
    pub fn is_active(&self) -> bool {
        self.window.lock().is_ok_and(|id| id.is_some())
    }

    pub fn start(&self, window: &Window, shortcut: Shortcut) -> Result<(), String> {
        let mut active = self.window.lock().map_err(error)?;
        shortcut.register(window.hwnd().map_err(error)?, FIRST_ID)?;
        *active = Some(FIRST_ID);
        Ok(())
    }

    pub fn start_playback(&self, hwnd: HWND) {
        self.unavailable_playback.get_or_init(|| {
            let modifiers = HOT_KEY_MODIFIERS(u32::from(CTRL_SHIFT)) | MOD_NOREPEAT;
            PLAYBACK_KEYS
                .iter()
                .enumerate()
                .filter_map(|(index, (key, label, _))| {
                    let id = FIRST_PLAYBACK_ID + index as i32;
                    unsafe { RegisterHotKey(Some(hwnd), id, modifiers, u32::from(key.0)) }
                        .err()
                        .map(|_| *label)
                })
                .collect()
        });
    }

    pub fn unavailable_playback(&self) -> &[&'static str] {
        self.unavailable_playback.get().map_or(&[], Vec::as_slice)
    }

    pub fn change(&self, window: &Window, shortcut: Option<Shortcut>) -> Result<(), String> {
        let hwnd = window.hwnd().map_err(error)?;
        let preferences = window.app_handle().state::<Preferences>();
        let mut settings = preferences.current.lock().map_err(error)?;
        let mut active = self.window.lock().map_err(error)?;
        if shortcut == settings.window_shortcut && (shortcut.is_none() || active.is_some()) {
            return Ok(());
        }

        // Keep the old registration until the replacement is both available and saved.
        let replacement = if let Some(shortcut) = shortcut {
            let id = if *active == Some(FIRST_ID) {
                SECOND_ID
            } else {
                FIRST_ID
            };
            shortcut.register(hwnd, id)?;
            Some(id)
        } else {
            None
        };
        let next = settings::Settings {
            window_shortcut: shortcut,
            ..*settings
        };
        if let Err(err) = settings::persist(&preferences.file, &next) {
            if let Some(id) = replacement {
                let _ = unsafe { UnregisterHotKey(Some(hwnd), id) };
            }
            return Err(error(err));
        }
        if let Some(id) = *active {
            let _ = unsafe { UnregisterHotKey(Some(hwnd), id) };
        }
        *active = replacement;
        *settings = next;
        Ok(())
    }

    pub fn handle(&self, window: &Window, id: usize) {
        if let Some((_, _, event)) = id
            .checked_sub(FIRST_PLAYBACK_ID as usize)
            .and_then(|index| PLAYBACK_KEYS.get(index))
        {
            if let Ok(view) = site::content(window.app_handle()) {
                let _ = view.eval(format!("window.dispatchEvent(new Event('{event}'))"));
            }
            return;
        }
        if self
            .window
            .lock()
            .is_ok_and(|active| active.is_some_and(|active| active as usize == id))
        {
            crate::toggle_window(window);
        }
    }
}

pub fn release(hwnd: HWND) {
    for id in [FIRST_ID, SECOND_ID]
        .into_iter()
        .chain(FIRST_PLAYBACK_ID..FIRST_PLAYBACK_ID + PLAYBACK_KEYS.len() as i32)
    {
        let _ = unsafe { UnregisterHotKey(Some(hwnd), id) };
    }
}
