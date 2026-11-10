use crate::{
    error,
    settings::{self, Preferences},
};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{Manager, Window};
use windows::Win32::{
    Foundation::HWND,
    UI::Input::KeyboardAndMouse::{
        RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_NOREPEAT,
    },
};

const FIRST_ID: i32 = 0x5101;
const SECOND_ID: i32 = 0x5102;

#[derive(Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
pub struct Shortcut {
    modifiers: u8,
    key: u8,
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
pub struct Hotkey(Mutex<Option<i32>>);

impl Hotkey {
    pub fn is_active(&self) -> bool {
        self.0.lock().is_ok_and(|id| id.is_some())
    }

    pub fn start(&self, window: &Window, shortcut: Shortcut) -> Result<(), String> {
        let mut active = self.0.lock().map_err(error)?;
        shortcut.register(window.hwnd().map_err(error)?, FIRST_ID)?;
        *active = Some(FIRST_ID);
        Ok(())
    }

    pub fn change(&self, window: &Window, shortcut: Option<Shortcut>) -> Result<(), String> {
        let hwnd = window.hwnd().map_err(error)?;
        let preferences = window.app_handle().state::<Preferences>();
        let mut settings = preferences.current.lock().map_err(error)?;
        let mut active = self.0.lock().map_err(error)?;
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
        if self
            .0
            .lock()
            .is_ok_and(|active| active.is_some_and(|active| active as usize == id))
        {
            crate::toggle_window(window);
        }
    }
}

pub fn release(hwnd: HWND) {
    for id in [FIRST_ID, SECOND_ID] {
        let _ = unsafe { UnregisterHotKey(Some(hwnd), id) };
    }
}
