use crate::settings;
use serde::{Deserialize, Serialize};
use std::{fs, mem::size_of, path::PathBuf, sync::Mutex};
use tauri::Window;
use windows::Win32::{
    Foundation::RECT,
    UI::WindowsAndMessaging::{
        GetWindowPlacement, SetWindowPlacement, SW_HIDE, SW_SHOWMAXIMIZED, WINDOWPLACEMENT,
        WPF_RESTORETOMAXIMIZED,
    },
};

#[derive(Clone, Copy, Deserialize, PartialEq, Serialize)]
struct Placement {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    maximized: bool,
}

impl Placement {
    fn valid(&self) -> bool {
        self.left < self.right
            && self.top < self.bottom
            && [self.left, self.top, self.right, self.bottom]
                .iter()
                .all(|value| (-1_000_000..=1_000_000).contains(value))
    }
}

pub struct WindowPlacement {
    file: PathBuf,
    saved: Mutex<Option<Placement>>,
}

impl WindowPlacement {
    pub fn load(file: PathBuf) -> Self {
        let saved = fs::read(&file)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Placement>(&bytes).ok())
            .filter(Placement::valid);
        Self {
            file,
            saved: Mutex::new(saved),
        }
    }

    pub fn restore(&self, window: &Window) -> bool {
        let Some(saved) = self.saved.lock().ok().and_then(|saved| *saved) else {
            return false;
        };
        let Ok(hwnd) = window.hwnd() else {
            return false;
        };
        // Keep Windows workspace coordinates intact. Windows repositions off-screen placements.
        let placement = WINDOWPLACEMENT {
            length: size_of::<WINDOWPLACEMENT>() as u32,
            showCmd: SW_HIDE.0 as u32,
            rcNormalPosition: RECT {
                left: saved.left,
                top: saved.top,
                right: saved.right,
                bottom: saved.bottom,
            },
            ..Default::default()
        };
        match unsafe { SetWindowPlacement(hwnd, &placement) } {
            Ok(()) => saved.maximized,
            Err(err) => {
                eprintln!("Could not restore window placement: {err}");
                false
            }
        }
    }

    pub fn save(&self, window: &Window) {
        let Ok(hwnd) = window.hwnd() else {
            return;
        };
        let mut native = WINDOWPLACEMENT {
            length: size_of::<WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };
        if unsafe { GetWindowPlacement(hwnd, &mut native) }.is_err() {
            return;
        }
        let RECT {
            left,
            top,
            right,
            bottom,
        } = native.rcNormalPosition;
        let placement = Placement {
            left,
            top,
            right,
            bottom,
            maximized: native.showCmd == SW_SHOWMAXIMIZED.0 as u32
                || native.flags.contains(WPF_RESTORETOMAXIMIZED),
        };
        let Ok(mut saved) = self.saved.lock() else {
            return;
        };
        if placement.valid() && *saved != Some(placement) {
            match settings::persist(&self.file, &placement) {
                Ok(()) => *saved = Some(placement),
                Err(err) => eprintln!("Could not save window placement: {err}"),
            }
        }
    }
}
