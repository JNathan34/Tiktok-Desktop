(() => {
  if (!["www.tiktok.com", "tiktok.com", "m.tiktok.com"].includes(location.hostname) || location.protocol !== "https:") return;

  let suspended = false;
  const pause = () => document.querySelectorAll("video, audio").forEach((media) => media.pause());
  window.addEventListener("desktop-media-suspend", () => { suspended = true; pause(); });
  window.addEventListener("desktop-media-restore", () => { suspended = false; });
  document.addEventListener("play", (event) => {
    if (suspended && event.target instanceof HTMLMediaElement) event.target.pause();
  }, true);

  function visibleVideo() {
    if (document.fullscreenElement instanceof HTMLVideoElement) return document.fullscreenElement;
    let best;
    let area = 0;
    for (const video of document.querySelectorAll("video")) {
      if (video.readyState < 1) continue;
      const box = video.getBoundingClientRect();
      const width = Math.min(box.right, innerWidth) - Math.max(0, box.left);
      const height = Math.min(box.bottom, innerHeight) - Math.max(0, box.top);
      if (width > 0 && height > 0 && width * height > area && video.checkVisibility()) {
        area = width * height;
        best = video;
      }
    }
    return best;
  }

  window.addEventListener("desktop-media-play", () => {
    if (!suspended) visibleVideo()?.play().catch(() => {});
  });
  window.addEventListener("desktop-media-pause", pause);

  window.addEventListener("keydown", (event) => {
    if (event.defaultPrevented || event.isComposing || event.repeat || event.ctrlKey || event.altKey || event.metaKey || event.shiftKey) return;
    if (event.composedPath().some((node) => node instanceof Element &&
        node.matches("input, textarea, select, button, a, video[controls], [role=button], [role=slider], [role=textbox], [contenteditable]:not([contenteditable=false])"))) return;
    const key = event.key.toLowerCase();
    if (key !== " " && key !== "m" && key !== "f") return;
    const video = visibleVideo();
    if (!video) return;
    event.preventDefault();
    if (key === "m") video.muted = !video.muted;
    else if (key === "f") {
      if (document.fullscreenElement) document.exitFullscreen().catch(() => {});
      else video.requestFullscreen().catch(() => {});
    } else if (video.paused && !suspended) video.play().catch(() => {});
    else video.pause();
  });
})();
