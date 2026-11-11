export {};

type Theme = "dark" | "light" | "system";
type Feed = "home" | "following";
type VideoFit = "site" | "fit" | "fill";
interface DisplaySettings {
  videoOnly: boolean;
  hideSidebar: boolean;
  hideActions: boolean;
  hideArrows: boolean;
  hideDetails: boolean;
  hideControls: boolean;
  zoomPercent: number;
  videoFit: VideoFit;
}

const defaultDisplay: DisplaySettings = {
  videoOnly: false,
  hideSidebar: false,
  hideActions: false,
  hideArrows: false,
  hideDetails: false,
  hideControls: false,
  zoomPercent: 100,
  videoFit: "site",
};

interface WindowShortcut {
  modifiers: number;
  key: number;
}

interface Settings {
  windowShortcut: WindowShortcut | null;
  shortcutActive: boolean;
  unavailablePlaybackShortcuts: string[];
  display: DisplaySettings;
  theme: Theme;
  pauseMinimized: boolean;
  hardwareAcceleration: boolean;
  launchFeed: Feed;
}

const tauri = (window as unknown as {
  __TAURI_INTERNALS__: { invoke<T>(name: string, args?: Record<string, unknown>): Promise<T> };
}).__TAURI_INTERNALS__;

function element<T extends HTMLElement = HTMLElement>(id: string): T {
  return document.getElementById(id) as T;
}

const theme = element<HTMLSelectElement>("theme");
const pause = element<HTMLInputElement>("pause");
const gpu = element<HTMLInputElement>("gpu");
const launch = element<HTMLSelectElement>("launch");
const confirmLogout = element("confirm-logout");
const notice = element("notice");
const noticeBar = element("notice-bar");
const displayToggles = [
  ["videoOnly", "video-only"],
  ["hideSidebar", "hide-sidebar"],
  ["hideActions", "hide-actions"],
  ["hideArrows", "hide-arrows"],
  ["hideDetails", "hide-details"],
  ["hideControls", "hide-controls"],
] as const;
const pageZoom = element<HTMLSelectElement>("page-zoom");
const videoFit = element<HTMLSelectElement>("video-fit");
const shortcutModifiers = element<HTMLSelectElement>("shortcut-modifiers");
const shortcutKey = element<HTMLSelectElement>("shortcut-key");
const shortcutStatus = element("shortcut-status");
const saveShortcut = element<HTMLButtonElement>("save-shortcut");
const clearShortcut = element<HTMLButtonElement>("clear-shortcut");
for (const label of "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789") {
  shortcutKey.add(new Option(label, String(label.charCodeAt(0))));
}
for (let number = 1; number <= 11; number++) {
  shortcutKey.add(new Option("F" + number, String(111 + number)));
}
shortcutKey.value = "84";

let settings: Settings | undefined;
let dataBusy = false;
let saveQueue: Promise<void> = Promise.resolve();

function showNotice(message: string) {
  notice.textContent = message;
  noticeBar.hidden = message === "";
}

function report(error: unknown) {
  showNotice(typeof error === "string" ? error : "The app could not complete that action.");
}

function renderShortcut() {
  if (!settings) return;
  const shortcut = settings.windowShortcut;
  clearShortcut.disabled = !shortcut;
  shortcutStatus.textContent = "Shortcut is off.";
  if (!shortcut) return;
  shortcutModifiers.value = String(shortcut.modifiers);
  shortcutKey.value = String(shortcut.key);
  const keys = [shortcut.modifiers & 2 ? "Ctrl" : "", shortcut.modifiers & 1 ? "Alt" : "",
    shortcut.modifiers & 4 ? "Shift" : "", shortcutKey.selectedOptions[0]?.text ?? ""].filter(Boolean).join(" + ");
  shortcutStatus.textContent = settings.shortcutActive ? "Active: " + keys : "Unavailable: " + keys;
}

async function changeShortcut(shortcut: WindowShortcut | null) {
  if (!settings) return;
  saveShortcut.disabled = clearShortcut.disabled = true;
  try {
    settings.windowShortcut = await tauri.invoke<WindowShortcut | null>("set_window_shortcut", { shortcut });
    settings.shortcutActive = settings.windowShortcut !== null;
    renderShortcut();
  } catch (error) {
    shortcutStatus.textContent = (typeof error === "string" ? error : "Could not change the shortcut.")
      + (settings.shortcutActive ? " Your previous shortcut is still active." : "");
    clearShortcut.disabled = !settings.windowShortcut;
  } finally {
    saveShortcut.disabled = false;
  }
}

saveShortcut.addEventListener("click", () => void changeShortcut({
  modifiers: Number(shortcutModifiers.value),
  key: Number(shortcutKey.value),
}));
clearShortcut.addEventListener("click", () => void changeShortcut(null));

function renderSettings() {
  if (!settings) return;
  const unavailable = settings.unavailablePlaybackShortcuts ?? [];
  element("playback-shortcut-status").textContent = unavailable.length
    ? "Windows could not register Ctrl + Shift with " + unavailable.join(", ") + "."
    : "Works from other apps while TikTok Desktop is running.";
  document.documentElement.dataset.theme = settings.theme;
  theme.value = settings.theme;
  pause.checked = settings.pauseMinimized;
  gpu.checked = settings.hardwareAcceleration;
  launch.value = settings.launchFeed;
  for (const [key, id] of displayToggles) {
    const toggle = element<HTMLInputElement>(id);
    toggle.checked = settings.display[key];
    toggle.disabled = key !== "videoOnly" && settings.display.videoOnly;
  }
  pageZoom.value = String(settings.display.zoomPercent);
  videoFit.value = settings.display.videoFit;
}

function update(patch: Partial<Settings>, display?: Partial<DisplaySettings>) {
  saveQueue = saveQueue.then(async () => {
    if (!settings) return;
    const next = { ...settings, ...patch, display: { ...settings.display, ...display } };
    try {
      const restart = await tauri.invoke<boolean>("save_settings", {
        settings: next, videoOnlyChanged: display?.videoOnly !== undefined,
      });
      settings = { ...next, windowShortcut: settings.windowShortcut, shortcutActive: settings.shortcutActive };
      element("gpu-note").textContent = restart
        ? "Restart TikTok Desktop to apply this change."
        : "Use the GPU when supported. Changes apply after restart.";
      showNotice("");
    } catch (error) {
      report(error);
    }
    renderSettings();
  });
}

async function clearData(signOut: boolean) {
  if (dataBusy) return;
  dataBusy = true;
  const cacheButton = element<HTMLButtonElement>("clear-cache");
  const logoutButton = element<HTMLButtonElement>("log-out");
  cacheButton.disabled = logoutButton.disabled = true;
  confirmLogout.hidden = true;
  showNotice(signOut ? "Removing saved TikTok data…" : "Clearing cached files…");
  try {
    await tauri.invoke("clear_tiktok_data", { signOut });
    showNotice(signOut ? "TikTok session removed from this app." : "Disk cache cleared.");
  } catch (error) {
    report(error);
  } finally {
    cacheButton.disabled = logoutButton.disabled = false;
    dataBusy = false;
  }
}

element("dismiss-notice").addEventListener("click", () => showNotice(""));
for (const [key, id] of displayToggles) {
  const toggle = element<HTMLInputElement>(id);
  toggle.addEventListener("change", () => update({}, { [key]: toggle.checked }));
}
pageZoom.addEventListener("change", () => update({}, { zoomPercent: Number(pageZoom.value) }));
videoFit.addEventListener("change", () => update({}, { videoFit: videoFit.value as VideoFit }));
element("reset-display").addEventListener("click", () => update({}, defaultDisplay));
theme.addEventListener("change", () => update({ theme: theme.value as Theme }));
pause.addEventListener("change", () => update({ pauseMinimized: pause.checked }));
gpu.addEventListener("change", () => update({ hardwareAcceleration: gpu.checked }));
launch.addEventListener("change", () => update({ launchFeed: launch.value as Feed }));
element("clear-cache").addEventListener("click", () => void clearData(false));
element("log-out").addEventListener("click", () => { confirmLogout.hidden = false; element("cancel-logout").focus(); });
element("cancel-logout").addEventListener("click", () => { confirmLogout.hidden = true; element("log-out").focus(); });
element("confirm-logout-button").addEventListener("click", () => void clearData(true));

window.addEventListener("desktop-video-only-changed", () => {
  saveQueue = saveQueue.then(async () => {
    try {
      const current = await tauri.invoke<Settings>("get_settings");
      if (settings) settings.display.videoOnly = current.display.videoOnly;
      renderSettings();
    } catch (error) {
      report(error);
    }
  });
});
window.addEventListener("keydown", event => {
  if (event.key === "Escape") {
    event.preventDefault();
    void tauri.invoke("close_settings").catch(report);
  }
});

tauri.invoke<Settings>("get_settings").then((value) => {
  settings = value;
  renderSettings();
  renderShortcut();
}).catch(report);
