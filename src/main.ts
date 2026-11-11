export {};

const tauri = (window as unknown as {
  __TAURI_INTERNALS__: { invoke<T>(name: string, args?: Record<string, unknown>): Promise<T> };
}).__TAURI_INTERNALS__;

function element<T extends HTMLElement = HTMLElement>(id: string): T {
  return document.getElementById(id) as T;
}

const sitePane = element("site-pane");
const notice = element("notice");
const noticeBar = element("notice-bar");
const qrButton = element<HTMLButtonElement>("open-qr-sign-in");
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
  const pane = { x: box.x, y: box.y, width: box.width, height: box.height };
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

qrButton.addEventListener("click", async () => {
  qrButton.disabled = true;
  try {
    await tauri.invoke("open_qr_sign_in");
    showNotice("");
    await placeSite(true);
  } catch (error) {
    report(error);
  } finally {
    qrButton.disabled = false;
  }
});
element("dismiss-notice").addEventListener("click", () => {
  showNotice("");
  void placeSite(true);
});
window.addEventListener("desktop-popup-blocked", () => {
  showNotice("This popup could not open. For TikTok sign-in, use a QR code.", true);
});
window.addEventListener("desktop-load-error", () => showNotice("WebView2 could not start the TikTok view."));
window.addEventListener("desktop-theme-changed", event => {
  document.documentElement.dataset.theme = (event as CustomEvent<string>).detail;
});
new ResizeObserver(scheduleLayout).observe(sitePane);
window.addEventListener("resize", scheduleLayout);

tauri.invoke<{ theme: string }>("get_settings").then(({ theme }) => {
  document.documentElement.dataset.theme = theme;
}).catch(report);
