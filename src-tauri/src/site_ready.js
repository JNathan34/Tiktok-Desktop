(() => {
  if (!document.body) return "blank";

  function visibleArea(element, minimum = 32) {
    const box = element.getBoundingClientRect();
    const width = Math.min(box.right, innerWidth) - Math.max(0, box.left);
    const height = Math.min(box.bottom, innerHeight) - Math.max(0, box.top);
    return width >= minimum && height >= minimum && element.checkVisibility({ visibilityProperty: true })
      ? width * height
      : 0;
  }

  let card;
  let cardArea = 0;
  for (const item of document.querySelectorAll('[data-e2e="recommend-list-item-container"]')) {
    const area = visibleArea(item);
    if (area > cardArea) {
      card = item;
      cardArea = area;
    }
  }
  const surface = card ?? document.querySelector("main") ?? document.body;
  const errorText = /having trouble playing this video/i;
  if (errorText.test(surface.innerText)) return "video-error";

  let video;
  let largest = 0;
  for (const candidate of surface.querySelectorAll("video")) {
    const area = visibleArea(candidate);
    if (area > largest) {
      video = candidate;
      largest = area;
    }
  }
  if (video) {
    if (video.error && video.error.code !== MediaError.MEDIA_ERR_ABORTED) return "video-error";
    if (video.readyState < HTMLMediaElement.HAVE_CURRENT_DATA ||
        (!video.paused && !video.ended && video.readyState < HTMLMediaElement.HAVE_FUTURE_DATA)) {
      return "buffering";
    }
    return "ready";
  }
  if ([...surface.querySelectorAll('[role="progressbar"], [aria-busy="true"]')].some(
    indicator => visibleArea(indicator, 8) > 0
  )) return "buffering";
  if (surface.innerText.trim() || [...surface.querySelectorAll("img")].some(img =>
    img.complete && img.naturalWidth > 0 && visibleArea(img) > 0
  )) return "ready";
  return "blank";
})()
