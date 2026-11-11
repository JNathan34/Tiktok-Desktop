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
const reloadButton = element<HTMLButtonElement>("reload-tiktok");
const dismissButton = element<HTMLButtonElement>("dismiss-notice");
let lastLayout = "";
let layoutFrame = 0;
let loadAttempt = 0;

function showNotice(message: string, { qr = false, reload = false, loading = false } = {}) {
  notice.textContent = message;
  noticeBar.hidden = message === "";
  qrButton.hidden = !qr;
  reloadButton.hidden = !reload;
  dismissButton.hidden = loading;
  scheduleLayout();
}

function report(error: unknown) {
  loadAttempt++;
  showNotice(typeof error === "string" ? error : "The app could not complete that action.");
}

async function watchFeed() {
  const attempt = ++loadAttempt;
  const start = performance.now();
  let stalled = false;
  showNotice("Loading TikTok…", { loading: true });
  while (attempt === loadAttempt && performance.now() - start < 60_000) {
    const ready = await tauri.invoke<boolean>("tiktok_has_content").catch(() => false);
    if (attempt !== loadAttempt) return;
    if (ready) {
      showNotice("");
      return;
    }
    if (!stalled && performance.now() - start >= 15_000) {
      stalled = true;
      showNotice("TikTok is taking longer to load.", { reload: true });
    }
    await new Promise(resolve => setTimeout(resolve, stalled ? 2500 : 750));
  }
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
reloadButton.addEventListener("click", async () => {
  reloadButton.disabled = true;
  loadAttempt++;
  try {
    await tauri.invoke("reload_tiktok");
    void watchFeed();
  } catch (error) {
    showNotice(String(error), { reload: true });
  } finally {
    reloadButton.disabled = false;
  }
});
dismissButton.addEventListener("click", () => {
  loadAttempt++;
  showNotice("");
  void placeSite(true);
});
window.addEventListener("desktop-popup-blocked", () => {
  loadAttempt++;
  showNotice("This popup could not open. For TikTok sign-in, use a QR code.", { qr: true });
});
window.addEventListener("desktop-load-error", () => {
  loadAttempt++;
  showNotice("WebView2 could not start the TikTok view.", { reload: true });
});
window.addEventListener("desktop-theme-changed", event => {
  document.documentElement.dataset.theme = (event as CustomEvent<string>).detail;
});
new ResizeObserver(scheduleLayout).observe(sitePane);
window.addEventListener("resize", scheduleLayout);
void watchFeed();

tauri.invoke<{ theme: string }>("get_settings").then(({ theme }) => {
  document.documentElement.dataset.theme = theme;
}).catch(report);
