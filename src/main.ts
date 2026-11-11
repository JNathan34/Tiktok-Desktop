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
const nextButton = element<HTMLButtonElement>("next-tiktok");
const dismissButton = element<HTMLButtonElement>("dismiss-notice");
let lastLayout = "";
let layoutFrame = 0;
let loadAttempt = 0;

function showNotice(message: string, { qr = false, reload = false, next = false, loading = false } = {}) {
  notice.textContent = message;
  noticeBar.hidden = message === "";
  qrButton.hidden = !qr;
  reloadButton.hidden = !reload;
  nextButton.hidden = !next;
  dismissButton.hidden = loading;
  scheduleLayout();
}

function report(error: unknown) {
  loadAttempt++;
  showNotice(typeof error === "string" ? error : "The app could not complete that action.");
}

type PlaybackHealth = "blank" | "buffering" | "inactive" | "ready" | "video-error";

async function watchFeed({ loading = false, autoRetry = false, dismissCurrent = false } = {}) {
  const attempt = ++loadAttempt;
  let waitingSince = performance.now();
  let previous: PlaybackHealth | undefined;
  let dismissedHealth: PlaybackHealth | undefined;
  if (loading) showNotice("Loading TikTok…", { loading: true });
  while (attempt === loadAttempt) {
    const health = await tauri.invoke<PlaybackHealth>("tiktok_health").catch(() => "blank" as const);
    if (attempt !== loadAttempt) return;
    const now = performance.now();
    if (health !== previous) waitingSince = now;

    if (health !== "inactive") {
      if (dismissCurrent) dismissedHealth = health;
      else if (health !== dismissedHealth) dismissedHealth = undefined;
      dismissCurrent = false;
    }

    if (health === "ready") {
      autoRetry = false;
      if (!noticeBar.hidden) showNotice("");
    } else if (health === "video-error" && health !== previous && health !== dismissedHealth) {
      autoRetry = false;
      showNotice("This video couldn’t play.", { reload: true, next: true });
    } else if (health !== dismissedHealth &&
               (health === "blank" || health === "buffering") && now - waitingSince >= 15_000) {
      if (autoRetry) {
        autoRetry = false;
        waitingSince = now;
        showNotice("Retrying TikTok…", { loading: true });
        try {
          await tauri.invoke("reload_tiktok");
        } catch {
          if (attempt === loadAttempt) showNotice("TikTok couldn’t reload.", { reload: true });
        }
      } else {
        const message = health === "buffering" ? "This video is still buffering." : "TikTok is taking longer to load.";
        if (notice.textContent !== message) showNotice(message, { reload: true, next: health === "buffering" });
      }
    }
    previous = health;
    await new Promise(resolve => setTimeout(resolve, health === "ready" || health === "inactive" ? 2500 : 1000));
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
    void watchFeed();
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
    void watchFeed({ loading: true });
  } catch (error) {
    showNotice(String(error), { reload: true });
  } finally {
    reloadButton.disabled = false;
  }
});
nextButton.addEventListener("click", async () => {
  try {
    await tauri.invoke("next_tiktok");
    showNotice("");
    await placeSite(true);
    void watchFeed();
  } catch (error) {
    showNotice(String(error), { reload: true, next: true });
  }
});
dismissButton.addEventListener("click", () => {
  const dismissCurrent = !reloadButton.hidden;
  showNotice("");
  void placeSite(true);
  void watchFeed({ dismissCurrent });
});
window.addEventListener("desktop-feed-reloaded", () => void watchFeed({ loading: true }));
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
void watchFeed({ loading: true, autoRetry: true });

tauri.invoke<{ theme: string }>("get_settings").then(({ theme }) => {
  document.documentElement.dataset.theme = theme;
}).catch(report);
