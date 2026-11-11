(() => {
  if (!document.body) return false;
  if (document.body.innerText.trim()) return true;
  return [...document.querySelectorAll("video, main img")].some(media => {
    const box = media.getBoundingClientRect();
    if (box.width < 32 || box.height < 32 || box.bottom <= 0 || box.right <= 0 ||
        box.top >= innerHeight || box.left >= innerWidth || !media.checkVisibility()) return false;
    return media instanceof HTMLVideoElement
      ? media.readyState >= 2 && media.videoWidth > 0
      : media.complete && media.naturalWidth > 0;
  });
})()
