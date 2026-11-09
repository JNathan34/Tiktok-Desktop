# TikTok Desktop

An independent Windows 11 client for the TikTok website. This project is unaffiliated with TikTok. The app uses a Rust/Tauri 2 host, two WebView2 surfaces in a single window, and a bundled TypeScript, HTML and CSS shell.

## Install

Download the installer for your PC from [GitHub Releases](https://github.com/JNathan34/tiktok-desktop-windows/releases/latest).

Choose the **ARM64** installer for Snapdragon Windows devices or the **x64** installer for Intel and AMD 64-bit Windows devices. Run the setup program and open **TikTok Desktop** from Start. Windows uses its Evergreen WebView2 Runtime. The setup program downloads the runtime bootstrapper if needed. The builds are unsigned and Windows may show a publisher warning.

Use TikTok’s own **Log in** control to sign in. The app keeps the site’s browser session in its own profile. Use TikTok’s embedded controls for playback, likes, comments, sharing and creator profiles. TikTok fills the area below the standard Windows title bar. Use TikTok’s own navigation.

In **Settings → Display**, **Video only** hides the feed controls and details together. The individual switches hide the sidebar, reactions, arrows, video details or playback bar. **Page zoom** changes the TikTok web view, and **Video sizing** can fit the whole video or fill the frame by cropping. **Reset display** restores TikTok's default layout.

## Controls

| Input | Action |
| --- | --- |
| Wheel, Up or Down | Navigate the TikTok feed using its native scrolling and keyboard behaviour |
| Space | Play or pause the visible video while the site has focus |
| M | Toggle mute |
| F / Esc | Enter or leave fullscreen |
| System tray → Settings or Ctrl+, | Open app settings |
| Alt+F4 | Close the window |

Playback keys defer to text inputs, interactive elements and site handlers. The app pauses HTML media while Settings is open. With **Pause when minimised** enabled, it also pauses on Windows minimise and leaves playback paused on restore. **Clear disk cache** preserves cookies and site storage. **Log out locally** clears the app’s TikTok profile after unloading the active site. The hardware acceleration preference applies at the next launch and WebView2 chooses the decoder for each video.

## Build from source

Install Node.js 22 or later, Rust stable with its MSVC toolchain, Microsoft C++ Build Tools and a Windows SDK. In Visual Studio Build Tools, include **Desktop development with C++**, the x64 compiler and the ARM64 compiler if building ARM64. Windows 11 normally already has the Evergreen WebView2 Runtime. Run each build in the appropriate Visual Studio Developer PowerShell so the Windows resource compiler and linker are on the path.

```powershell
npm ci
rustup target add x86_64-pc-windows-msvc aarch64-pc-windows-msvc
npm run build
npm run tauri -- build --target x86_64-pc-windows-msvc
npm run tauri -- build --target aarch64-pc-windows-msvc
```

Build output is under `src-tauri/target/<target>/release/`. The NSIS installer is under `bundle/nsis/` inside that release folder. To run the Rust security test on your native architecture, build the frontend first and run `cargo test --manifest-path src-tauri/Cargo.toml`. To launch a development build with the same bundled-origin restrictions, use `npm run tauri -- dev --no-dev-server`.

- [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/#windows)
- [Tauri Windows installer documentation](https://v2.tauri.app/distribute/windows-installer/)
- [Microsoft Evergreen WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/)

## Site and security boundaries

- The website controls feed availability, account actions, challenges, region restrictions and login methods. External OAuth windows and off-site links are blocked. For a Google-linked account, open TikTok’s QR sign-in and scan the code with the TikTok app on a phone already signed into that account. TikTok’s phone or email sign-in is also available for accounts with those methods set up.
- The layout preserves TikTok’s own player and interaction UI, including the navigation the site chooses to render. The shell follows your theme and sends a colour preference to WebView2. TikTok may apply its own saved theme.
- The local shell uses an in-private WebView2 profile. TikTok uses the persistent profile, with one WebView2 browser environment shared between the surfaces. The site may request subresources from its media and service hosts. Top-level navigation is limited to known TikTok HTTPS hosts and the neutral blank document used for startup and local sign-out.
- Only the bundled shell webview receives the four declared Tauri command permissions. Each command also checks the caller’s webview label and local origin. WebView2 web messaging is disabled on the remote controller. Website-initiated downloads and requests for browser permissions are denied. Same-site popups with a known URL can open in the existing site surface.
- Site routes are in `src-tauri/src/site.rs`. Playback shortcuts use standard media elements in `playback.js`. These are the areas to check after a TikTok layout change.






Windows media controls use the TikTok Desktop playback session. Play and pause apply to the visible TikTok video.
