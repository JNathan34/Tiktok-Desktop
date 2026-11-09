use serde::Deserialize;
use tauri::Webview;
use webview2_com::{
    CallDevToolsProtocolMethodCompletedHandler, CoTaskMemPWSTR,
    DevToolsProtocolEventReceivedEventHandler, ExecuteScriptCompletedHandler,
    Microsoft::Web::WebView2::Win32::ICoreWebView2, NavigationCompletedEventHandler,
};
use windows::{
    core::{factory, HSTRING, PWSTR},
    Foundation::TypedEventHandler,
    Media::{
        MediaPlaybackStatus, MediaPlaybackType, SystemMediaTransportControls,
        SystemMediaTransportControlsButton, SystemMediaTransportControlsButtonPressedEventArgs,
    },
    Win32::{Foundation::HWND, System::WinRT::ISystemMediaTransportControlsInterop},
};

#[derive(Deserialize)]
struct Playback {
    playing: bool,
    title: String,
}

fn refresh(
    core: &ICoreWebView2,
    controls: &SystemMediaTransportControls,
) -> windows::core::Result<()> {
    let controls = controls.clone();
    let complete = ExecuteScriptCompletedHandler::create(Box::new(move |status, json| {
        status?;
        let playback = serde_json::from_str::<Option<Playback>>(&json)
            .ok()
            .flatten();
        if let Some(playback) = playback {
            let display = controls.DisplayUpdater()?;
            display.SetType(MediaPlaybackType::Video)?;
            display
                .VideoProperties()?
                .SetTitle(&HSTRING::from(playback.title))?;
            display.Update()?;
            controls.SetPlaybackStatus(if playback.playing {
                MediaPlaybackStatus::Playing
            } else {
                MediaPlaybackStatus::Paused
            })?;
            controls.SetIsEnabled(true)?;
        } else {
            controls.SetIsEnabled(false)?;
        }
        Ok(())
    }));
    unsafe { core.ExecuteScript(&HSTRING::from(include_str!("media_state.js")), &complete) }
}

pub fn attach(hwnd: HWND, core: &ICoreWebView2, view: Webview) -> windows::core::Result<()> {
    let interop = factory::<SystemMediaTransportControls, ISystemMediaTransportControlsInterop>()?;
    let controls: SystemMediaTransportControls = unsafe { interop.GetForWindow(hwnd)? };
    controls.SetIsEnabled(false)?;
    controls.SetIsPlayEnabled(true)?;
    controls.SetIsPauseEnabled(true)?;
    controls.ButtonPressed(&TypedEventHandler::<
        SystemMediaTransportControls,
        SystemMediaTransportControlsButtonPressedEventArgs,
    >::new(move |_, args| {
        let Some(args) = args.as_ref() else {
            return Ok(());
        };
        let script = match args.Button()? {
            SystemMediaTransportControlsButton::Play => {
                "window.dispatchEvent(new Event('desktop-media-play'))"
            }
            SystemMediaTransportControlsButton::Pause => {
                "window.dispatchEvent(new Event('desktop-media-pause'))"
            }
            _ => return Ok(()),
        };
        let _ = view.eval(script);
        Ok(())
    }))?;

    let media_controls = controls.clone();
    let media_event =
        DevToolsProtocolEventReceivedEventHandler::create(Box::new(move |core, args| {
            let (Some(core), Some(args)) = (core, args) else {
                return Ok(());
            };
            let mut value = PWSTR::null();
            unsafe { args.ParameterObjectAsJson(&mut value)? };
            let event = CoTaskMemPWSTR::from(value).to_string();
            if ["kPlay", "kPause", "kEnded", "kWebMediaPlayerDestroyed"]
                .iter()
                .any(|name| event.contains(name))
            {
                refresh(&core, &media_controls)?;
            }
            Ok(())
        }));
    let loaded = NavigationCompletedEventHandler::create(Box::new(move |core, _| {
        if let Some(core) = core {
            refresh(&core, &controls)?;
        }
        Ok(())
    }));
    let enabled = CallDevToolsProtocolMethodCompletedHandler::create(Box::new(|status, _| status));
    unsafe {
        let receiver =
            core.GetDevToolsProtocolEventReceiver(&HSTRING::from("Media.playerEventsAdded"))?;
        receiver.add_DevToolsProtocolEventReceived(&media_event, &mut 0)?;
        core.add_NavigationCompleted(&loaded, &mut 0)?;
        core.CallDevToolsProtocolMethod(
            &HSTRING::from("Media.enable"),
            &HSTRING::from("{}"),
            &enabled,
        )
    }
}
