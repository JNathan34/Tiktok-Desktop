(() => {
  if (location.protocol !== "https:" || !["www.tiktok.com", "tiktok.com", "m.tiktok.com"].includes(location.hostname)) return null;
  const videos = [...document.querySelectorAll("video")].filter((video) => video.readyState >= 1 && !video.ended);
  if (videos.length === 0) return null;
  return {
    playing: videos.some((video) => !video.paused),
    title: (navigator.mediaSession?.metadata?.title || document.title || "TikTok").slice(0, 160),
  };
})()
