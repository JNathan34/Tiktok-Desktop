use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path, sync::Mutex};

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
    System,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Feed {
    Home,
    Following,
}

#[derive(Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoFit {
    #[default]
    Site,
    Fit,
    Fill,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DisplaySettings {
    pub video_only: bool,
    pub hide_sidebar: bool,
    pub hide_actions: bool,
    pub hide_arrows: bool,
    pub hide_details: bool,
    pub hide_controls: bool,
    pub zoom_percent: u16,
    pub video_fit: VideoFit,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            video_only: false,
            hide_sidebar: false,
            hide_actions: false,
            hide_arrows: false,
            hide_details: false,
            hide_controls: false,
            zoom_percent: 100,
            video_fit: VideoFit::Site,
        }
    }
}

impl DisplaySettings {
    pub fn zoom_factor(self) -> f64 {
        f64::from(self.zoom_percent.clamp(50, 175)) / 100.0
    }
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub theme: Theme,
    pub pause_minimized: bool,
    pub always_on_top: bool,
    pub hide_from_taskbar: bool,
    pub window_shortcut: Option<crate::hotkey::Shortcut>,
    pub hardware_acceleration: bool,
    pub launch_feed: Feed,
    pub display: DisplaySettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            pause_minimized: true,
            always_on_top: false,
            hide_from_taskbar: false,
            window_shortcut: None,
            hardware_acceleration: true,
            launch_feed: Feed::Home,
            display: DisplaySettings::default(),
        }
    }
}

pub struct Preferences {
    pub file: std::path::PathBuf,
    pub current: Mutex<Settings>,
    pub startup_gpu: bool,
}

impl Preferences {
    pub fn load(file: std::path::PathBuf) -> Self {
        let settings: Settings = fs::read(&file)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self {
            file,
            current: Mutex::new(settings),
            startup_gpu: settings.hardware_acceleration,
        }
    }
}

pub fn persist(path: &Path, settings: &Settings) -> io::Result<()> {
    let next = path.with_extension("tmp");
    fs::write(&next, serde_json::to_vec_pretty(settings)?)?;
    fs::rename(next, path)
}
