use std::{fs::File, io::BufReader, os::windows::ffi::OsStrExt, path::Path};
use tauri::{image::Image, Window};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{LPARAM, WPARAM},
        UI::WindowsAndMessaging::{
            DestroyIcon, GetSystemMetrics, LoadImageW, SendMessageW, HICON, ICON_BIG, IMAGE_ICON,
            LR_LOADFROMFILE, SM_CXICON, SYSTEM_METRICS_INDEX, WM_SETICON,
        },
    },
};

pub fn load(path: &Path) -> Option<ico::IconDir> {
    let file = File::open(path).ok()?;
    if file.metadata().ok()?.len() > 1024 * 1024 {
        return None;
    }
    let icon = ico::IconDir::read(BufReader::new(file)).ok()?;
    (icon.resource_type() == ico::ResourceType::Icon).then_some(icon)
}

pub fn system_size(metric: SYSTEM_METRICS_INDEX) -> u32 {
    unsafe { GetSystemMetrics(metric) }.max(16) as u32
}

pub fn frame(icon: &ico::IconDir, size: u32) -> Option<Image<'static>> {
    let frame = icon
        .entries()
        .iter()
        .filter(|entry| entry.width() == entry.height() && entry.width() <= 256)
        .min_by_key(|entry| (entry.width() < size, entry.width().abs_diff(size)))?
        .decode()
        .ok()?;
    Some(Image::new_owned(
        frame.rgba_data().to_vec(),
        frame.width(),
        frame.height(),
    ))
}

pub struct TaskbarIcon(isize);

impl Drop for TaskbarIcon {
    fn drop(&mut self) {
        let _ = unsafe { DestroyIcon(HICON(self.0 as *mut _)) };
    }
}

pub fn set_taskbar(window: &Window, path: &Path) -> Option<TaskbarIcon> {
    let hwnd = window.hwnd().ok()?;
    let name: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let size = system_size(SM_CXICON) as i32;
    let handle = unsafe {
        LoadImageW(
            None,
            PCWSTR(name.as_ptr()),
            IMAGE_ICON,
            size,
            size,
            LR_LOADFROMFILE,
        )
        .ok()?
    };
    let icon = TaskbarIcon(handle.0 as isize);
    unsafe {
        SendMessageW(
            hwnd,
            WM_SETICON,
            Some(WPARAM(ICON_BIG as usize)),
            Some(LPARAM(icon.0)),
        );
    }
    Some(icon)
}
