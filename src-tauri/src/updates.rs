use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::{Update, UpdaterExt};

#[derive(Default)]
pub struct UpdateState(AtomicBool);

pub fn check(app: &AppHandle, manual: bool) {
    if app.state::<UpdateState>().0.swap(true, Ordering::AcqRel) {
        return;
    }

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = async {
            app.updater_builder()
                .timeout(Duration::from_secs(40))
                .build()?
                .check()
                .await
        }
        .await;

        match result {
            Ok(Some(update)) => offer_update(app, update),
            Ok(None) => {
                if manual {
                    let version = &app.package_info().version;
                    notify(
                        &app,
                        format!("TikTok Desktop {version} is up to date."),
                        MessageDialogKind::Info,
                    );
                }
                finish(&app);
            }
            Err(error) => {
                eprintln!("Update check failed: {error}");
                if manual {
                    notify(
                        &app,
                        "Could not check GitHub Releases. Try again from the tray.",
                        MessageDialogKind::Error,
                    );
                }
                finish(&app);
            }
        }
    });
}

fn offer_update(app: AppHandle, update: Update) {
    let Some(window) = app.get_window("desktop") else {
        finish(&app);
        return;
    };

    let prompt = app
        .dialog()
        .message(format!(
            "TikTok Desktop {} is available. The app will restart after the update is installed.",
            update.version
        ))
        .title("TikTok Desktop update")
        .parent(&window)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Install update".into(),
            "Later".into(),
        ));
    prompt.show(move |accepted| {
        if accepted {
            crate::remember_window(&window);
            tauri::async_runtime::spawn(async move {
                if let Err(error) = update.download_and_install(|_, _| {}, || {}).await {
                    eprintln!("Update installation failed: {error}");
                    notify(&app, "Could not download, verify, or install the update. Try again from the tray.", MessageDialogKind::Error);
                }
                finish(&app);
            });
        } else {
            finish(&app);
        }
    });
}

fn notify(app: &AppHandle, message: impl Into<String>, kind: MessageDialogKind) {
    if let Some(window) = app.get_window("desktop") {
        app.dialog()
            .message(message)
            .title("TikTok Desktop updates")
            .kind(kind)
            .parent(&window)
            .show(|_| {});
    }
}

fn finish(app: &AppHandle) {
    app.state::<UpdateState>().0.store(false, Ordering::Release);
}
