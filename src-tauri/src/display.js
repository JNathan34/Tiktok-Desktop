(settings) => {
  if (location.protocol !== "https:" || !["www.tiktok.com", "tiktok.com", "m.tiktok.com"].includes(location.hostname)) return;

  const styleKey = Symbol.for("tiktok-desktop.display");
  if (!document[styleKey]) {
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(`
      html[data-ttdesk-sidebar] :is([class*="--DivSideNavPlaceholderContainer"], [class*="--DivSideNavContainer"], [data-e2e="side-nav-skeleton"]),
      html[data-ttdesk-actions] :is([data-e2e="anchoring-engagement-vertical"], [data-e2e="like-icon"], [data-e2e="comment-icon"], [data-e2e="favorite-icon"], [data-e2e="share-icon"]),
      html[data-ttdesk-arrows] :is([data-e2e="anchoring-scroll-controls"], [data-e2e="feed-navigation-next"], [data-e2e="feed-navigation-prev"], [data-e2e="feed-navigation-previous"]),
      html[data-ttdesk-details] [data-e2e="anchoring-description-layer"],
      html[data-ttdesk-controls] [data-e2e="anchoring-playback-chrome"],
      html[data-ttdesk-video-only] [data-e2e="anchoring-ambient"] {
        display: none !important;
      }

      html[data-ttdesk-video-only] :is([data-e2e="anchoring-player-column"], [data-e2e="feed-video"]) {
        border-radius: 0 !important;
        background-color: black !important;
      }

      html:is([data-ttdesk-video-fit="fit"], [data-ttdesk-video-fit="fill"]) [data-e2e="feed-video"] .tiktok-web-player video {
        position: absolute !important;
        inset: 0 !important;
        width: 100% !important;
        height: 100% !important;
        max-width: none !important;
        max-height: none !important;
        transform: none !important;
        object-position: center !important;
        object-fit: contain !important;
      }

      html[data-ttdesk-video-fit="fill"] [data-e2e="feed-video"] .tiktok-web-player video {
        object-fit: cover !important;
      }
    `);
    document.adoptedStyleSheets = [...document.adoptedStyleSheets, sheet];
    document[styleKey] = sheet;
  }

  const root = document.documentElement;
  root.toggleAttribute("data-ttdesk-video-only", settings.videoOnly);
  root.toggleAttribute("data-ttdesk-sidebar", settings.videoOnly || settings.hideSidebar);
  root.toggleAttribute("data-ttdesk-actions", settings.videoOnly || settings.hideActions);
  root.toggleAttribute("data-ttdesk-arrows", settings.videoOnly || settings.hideArrows);
  root.toggleAttribute("data-ttdesk-details", settings.videoOnly || settings.hideDetails);
  root.toggleAttribute("data-ttdesk-controls", settings.videoOnly || settings.hideControls);
  root.dataset.ttdeskVideoFit = settings.videoOnly && settings.videoFit === "site" ? "fit" : settings.videoFit;
}
