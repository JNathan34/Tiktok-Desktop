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

interface Settings {
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

const sitePane = element("site-pane");
const settingsPane = element("settings-pane");
const theme = element<HTMLSelectElement>("theme");
const pause = element<HTMLInputElement>("pause");
const gpu = element<HTMLInputElement>("gpu");
const launch = element<HTMLSelectElement>("launch");
const confirmLogout = element("confirm-logout");
const notice = element("notice");
const noticeBar = element("notice-bar");
const qrButton = element<HTMLButtonElement>("open-qr-sign-in");
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

let settingsOpen = false;
let settings: Settings | undefined;
let dataBusy = false;
let saveQueue: Promise<void> = Promise.resolve();
let lastLayout = "";
let layoutFrame = 0;

function showNotice(message: string, offerQrSignIn = false) {
  notice.textContent = message;
  noticeBar.hidden = message === "";
  qrButton.hidden = !offerQrSignIn;
  scheduleLayout();
}

function report(error: unknown) {
  showNotice(typeof error === "string" ? error : "The app could not complete that action.");
}

async function placeSite(focus = false) {
  const box = sitePane.getBoundingClientRect();
  const pane = !settingsOpen
    ? { x: box.x, y: box.y, width: box.width, height: box.height }
    : null;
  const next = JSON.stringify(pane);
  if (next === lastLayout && !focus) return;
  lastLayout = next;
  try {
    await tauri.invoke("layout", { pane, focus });
  } catch (error) {
    lastLayout = "";
    report(error);
  }
}

function scheduleLayout() {
  if (layoutFrame) return;
  layoutFrame = requestAnimationFrame(() => {
    layoutFrame = 0;
    void placeSite();
  });
}

async function showSettings(open: boolean) {
  settingsOpen = open;
  settingsPane.hidden = !open;
  confirmLogout.hidden = true;
  showNotice("");
  await placeSite(true);
  if (settingsOpen && open) element("settings-title").focus();
}

function renderSettings() {
  if (!settings) return;
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
      const restart = await tauri.invoke<boolean>("save_settings", { settings: next });
      settings = next;
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

qrButton.addEventListener("click", async () => {
  qrButton.disabled = true;
  try {
    await tauri.invoke("open_qr_sign_in");
    await showSettings(false);
  } catch (error) {
    report(error);
  } finally {
    qrButton.disabled = false;
  }
});
element("dismiss-notice").addEventListener("click", () => {
  showNotice("");
  if (!settingsOpen) void placeSite(true);
});
for (const button of document.querySelectorAll("[data-return]")) {
  button.addEventListener("click", () => void showSettings(false));
}
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

window.addEventListener("desktop-settings", () => void showSettings(true));
window.addEventListener("desktop-popup-blocked", () => {
  showNotice("This popup could not open. For TikTok sign-in, use a QR code.", true);
});
window.addEventListener("desktop-load-error", () => showNotice("WebView2 could not start the TikTok view."));
window.addEventListener("keydown", (event) => {
  if (settingsOpen && event.key === "Escape") {
    event.preventDefault();
    void showSettings(false);
  }
});
new ResizeObserver(scheduleLayout).observe(sitePane);
window.addEventListener("resize", scheduleLayout);

tauri.invoke<Settings>("get_settings").then((value) => {
  settings = value;
  renderSettings();
}).catch(report);
