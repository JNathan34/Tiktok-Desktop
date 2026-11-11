export {};

interface TrayState {
  theme: "dark" | "light" | "system";
  videoOnly: boolean;
  alwaysOnTop: boolean;
  hideFromTaskbar: boolean;
}

interface TrayBrand {
  version: string;
  width: number;
  height: number;
  pixels: number[];
}

const { __TAURI_INTERNALS__: tauri, trayBrand } = window as unknown as {
  __TAURI_INTERNALS__: { invoke<T>(name: string, args?: Record<string, string>): Promise<T> };
  trayBrand: TrayBrand;
};
const items = [...document.querySelectorAll<HTMLButtonElement>("[data-action]")];
const toggles = document.querySelectorAll<HTMLButtonElement>('[role="menuitemcheckbox"]');
const status = document.querySelector<HTMLElement>("#tray-error")!;
let busy = false;
let active = 0;

const canvas = document.querySelector<HTMLCanvasElement>("#brand-icon")!;
canvas.width = trayBrand.width;
canvas.height = trayBrand.height;
canvas.getContext("2d")?.putImageData(
  new ImageData(new Uint8ClampedArray(trayBrand.pixels), trayBrand.width, trayBrand.height), 0, 0,
);
document.querySelector("#version")!.textContent = "v" + trayBrand.version;

function render(state: TrayState) {
  document.documentElement.dataset.theme = state.theme;
  for (const button of toggles) {
    const key = button.dataset.action as "videoOnly" | "alwaysOnTop" | "hideFromTaskbar";
    button.setAttribute("aria-checked", String(state[key]));
    button.querySelector(".toggle span")!.textContent = state[key] ? "On" : "Off";
  }
}

function select(index: number) {
  items[active].tabIndex = -1;
  active = (index + items.length) % items.length;
  items[active].tabIndex = 0;
  items[active].focus();
}

function showError(error: unknown) {
  status.textContent = String(error);
  status.hidden = false;
}

async function refresh() {
  status.hidden = true;
  try {
    render(await tauri.invoke<TrayState>("tray_state"));
  } catch (error) {
    showError(error);
  }
}

async function run(action: string) {
  if (busy) return;
  busy = true;
  status.hidden = true;
  try {
    render(await tauri.invoke<TrayState>("tray_action", { action }));
  } catch (error) {
    showError(error);
  } finally {
    busy = false;
  }
}

items.forEach((button, index) => {
  button.addEventListener("click", () => {
    select(index);
    void run(button.dataset.action!);
  });
});
document.addEventListener("keydown", event => {
  switch (event.key) {
    case "ArrowDown": select(active + 1); break;
    case "ArrowUp": select(active - 1); break;
    case "Home": select(0); break;
    case "End": select(items.length - 1); break;
    case "Tab": select(active + (event.shiftKey ? -1 : 1)); break;
    case "Escape": void run("dismiss"); break;
    default: return;
  }
  event.preventDefault();
});
document.addEventListener("contextmenu", event => event.preventDefault());
window.addEventListener("tray-opening", () => {
  select(0);
  void refresh();
});
select(0);
void refresh();
