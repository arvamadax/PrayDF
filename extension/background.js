chrome.action.onClicked.addListener(() => chrome.tabs.create({ url: "viewer.html" }));

// Right-click a PDF link -> hand its URL to the PrayDF desktop app through the praydf: protocol
chrome.runtime.onInstalled.addListener(() =>
  chrome.contextMenus.create({
    id: "open-in-app",
    title: "Buka di aplikasi PrayDF",
    contexts: ["link"],
    targetUrlPatterns: ["*://*/*.pdf", "*://*/*.pdf?*", "*://*/*.pdf#*", "*://*/*.PDF", "*://*/*.PDF?*"],
  }),
);

// A tab that receives a PDF (e.g. "Print" in Google Classroom) opens in our viewer instead, like Chrome's PDF viewer
const redirected = new Set(); // tab + URL already sent to the viewer, so Back shows Brave's viewer instead of bouncing
chrome.webRequest.onHeadersReceived.addListener(
  ({ tabId, url, method, statusCode, responseHeaders }) => {
    const header = (n) => responseHeaders?.find((h) => h.name.toLowerCase() === n)?.value?.toLowerCase() ?? "";
    if (tabId < 0 || method !== "GET" || statusCode !== 200) return;
    if (!/^application\/(x-)?pdf/.test(header("content-type")) || header("content-disposition").startsWith("attachment")) return;
    const key = `${tabId} ${url}`;
    if (redirected.has(key)) return;
    redirected.add(key);
    chrome.tabs.update(tabId, { url: chrome.runtime.getURL("viewer.html#" + encodeURIComponent(url)) });
  },
  { urls: ["http://*/*", "https://*/*"], types: ["main_frame"] },
  ["responseHeaders"],
);

chrome.contextMenus.onClicked.addListener((info, tab) => {
  // Navigating the current tab to an external protocol only shows Brave's "open app?" prompt; the page stays
  if (info.menuItemId === "open-in-app" && tab?.id >= 0) chrome.tabs.update(tab.id, { url: "praydf:" + encodeURIComponent(info.linkUrl) });
});
