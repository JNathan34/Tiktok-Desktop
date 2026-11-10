(() => {
  if (!["www.tiktok.com", "tiktok.com", "m.tiktok.com"].includes(location.hostname) || location.protocol !== "https:") return;

  let suspended = false;
  const pause = () => document.querySelectorAll("video, audio").forEach((media) => media.pause());
  window.addEventListener("desktop-media-suspend", () => { suspended = true; pause(); });
  window.addEventListener("desktop-media-restore", () => { suspended = false; });
  document.addEventListener("play", (event) => {
    if (suspended && event.target instanceof HTMLMediaElement) event.target.pause();
  }, true);

  function visibleArea(element) {
    const box = element.getBoundingClientRect();
    const width = Math.min(box.right, innerWidth) - Math.max(0, box.left);
    const height = Math.min(box.bottom, innerHeight) - Math.max(0, box.top);
    return width > 0 && height > 0 && element.checkVisibility() ? width * height : 0;
  }

  function visibleVideo() {
    if (document.fullscreenElement instanceof HTMLVideoElement) return document.fullscreenElement;
    let best;
    let area = 0;
    let inFeed = false;
    for (const video of document.querySelectorAll("video")) {
      if (video.readyState < 1) continue;
      const size = visibleArea(video);
      if (!size) continue;
      const isFeedVideo = Boolean(video.closest('[data-e2e="feed-video"]'));
      if ((isFeedVideo && !inFeed) || (isFeedVideo === inFeed && size > area)) {
        inFeed = isFeedVideo;
        area = size;
        best = video;
      }
    }
    return best;
  }

  window.addEventListener("desktop-media-play", () => {
    if (!suspended) visibleVideo()?.play().catch(() => {});
  });
  window.addEventListener("desktop-media-pause", pause);

  function stepFeed(direction) {
    const selectors = direction > 0
      ? '[data-e2e="feed-navigation-next"]'
      : '[data-e2e="feed-navigation-prev"], [data-e2e="feed-navigation-previous"]';
    const buttons = [...document.querySelectorAll(`:is(${selectors}):not(:disabled):not([aria-disabled=true])`)];
    const button = buttons.find((candidate) => candidate.checkVisibility()) ?? buttons[0];
    if (button) {
      button.click();
      return;
    }

    const feed = document.querySelector('[class*="--DivColumnListContainer"]');
    const items = [...(feed ?? document).querySelectorAll('[data-e2e="recommend-list-item-container"]')];
    let active = -1;
    let area = 0;
    items.forEach((item, index) => {
      const size = visibleArea(item);
      if (size > area) {
        area = size;
        active = index;
      }
    });
    if (active < 0) return;
    const target = items[active + direction];
    if (target) target.scrollIntoView({ block: "start", inline: "nearest", behavior: "instant" });
    else feed?.scrollBy({ top: direction * feed.clientHeight, behavior: "instant" });
  }

  function seek(seconds) {
    const video = visibleVideo();
    if (!video || !Number.isFinite(video.duration) || !video.seekable.length) return;
    const desired = video.currentTime + seconds;
    let nearest = video.seekable.start(0);
    for (let index = 0; index < video.seekable.length; index++) {
      const candidate = Math.max(video.seekable.start(index), Math.min(desired, video.seekable.end(index)));
      if (Math.abs(candidate - desired) < Math.abs(nearest - desired)) nearest = candidate;
    }
    video.currentTime = nearest;
  }

  window.addEventListener("desktop-media-previous", () => stepFeed(-1));
  window.addEventListener("desktop-media-next", () => stepFeed(1));
  window.addEventListener("desktop-media-rewind", () => seek(-5));
  window.addEventListener("desktop-media-forward", () => seek(5));

  const controls = 'button, a, [role="button"], [role="link"], [role="menuitem"], [role="checkbox"], [role="switch"], [role="tab"]';
  let pointerControl;
  window.addEventListener("pointerdown", (event) => {
    pointerControl = event.composedPath().find((node) => node instanceof Element && node.matches(controls));
  }, true);

  // Claim Space before the site's handlers can scroll or act on a mouse-focused button.
  window.addEventListener("keydown", (event) => {
    if (event.key === "Tab") pointerControl = undefined;
    if (event.code !== "Space" && event.key !== " ") return;
    if (suspended || event.defaultPrevented || event.isComposing ||
        event.ctrlKey || event.altKey || event.metaKey || event.shiftKey) return;

    const path = event.composedPath().filter((node) => node instanceof Element);
    if (path.some((node) => node.matches(
      'input, textarea, select, video[controls], [contenteditable]:not([contenteditable=false]), [role="textbox"], [role="searchbox"], [role="slider"], [role="dialog"], [role="alertdialog"], dialog[open]'
    ))) return;
    if (!path.includes(pointerControl) && path.some((node) => node.matches(controls))) return;

    const video = visibleVideo();
    if (!video) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    if (event.repeat) return;

    const playButton = video.closest('[data-e2e="feed-video"]')
      ?.querySelector('button[data-e2e="anchoring-playback-control-play"]:enabled');
    if (playButton) playButton.click();
    else if (video.paused) video.play().catch(() => {});
    else video.pause();
  }, true);

  window.addEventListener("keydown", (event) => {
    if (event.defaultPrevented || event.isComposing || event.repeat || event.ctrlKey || event.altKey || event.metaKey || event.shiftKey) return;
    if (event.composedPath().some((node) => node instanceof Element &&
        node.matches("input, textarea, select, button, a, video[controls], [role=button], [role=slider], [role=textbox], [contenteditable]:not([contenteditable=false])"))) return;
    const key = event.key.toLowerCase();
    if (key !== "m" && key !== "f") return;
    const video = visibleVideo();
    if (!video) return;
    event.preventDefault();
    if (key === "m") video.muted = !video.muted;
    else if (key === "f") {
      if (document.fullscreenElement) document.exitFullscreen().catch(() => {});
      else video.requestFullscreen().catch(() => {});
    }
  });
})();
