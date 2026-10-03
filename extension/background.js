const BITS_BRIDGE_URL = "http://127.0.0.1:6801";

// Create context menu on install
chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({
    id: "download-with-bits",
    title: "Download with Bits Download Manager",
    contexts: ["link", "image", "video", "audio"]
  });
});

// Handle context menu clicks
chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  if (info.menuItemId === "download-with-bits") {
    const url = info.linkUrl || info.srcUrl || info.pageUrl;
    if (!url) return;

    const referer = tab?.url || "";
    let cookies = "";
    try {
      if (tab?.url) {
        const cookieList = await chrome.cookies.getAll({ url: tab.url });
        cookies = cookieList.map((c) => `${c.name}=${c.value}`).join("; ");
      }
    } catch {
      // ignore
    }

    await sendToBits({
      url,
      referer,
      cookies,
      userAgent: navigator.userAgent
    });
  }
});

// Optional automatic browser download interception
chrome.downloads.onCreated.addListener(async (item) => {
  const settings = await chrome.storage.local.get({ intercept: false });
  if (!settings.intercept || !item.url) return;

  // Don't intercept blob/data urls or extension URLs
  if (item.url.startsWith("blob:") || item.url.startsWith("data:") || item.url.startsWith("chrome-extension:")) {
    return;
  }

  // Cancel standard browser download and forward to Bits
  try {
    chrome.downloads.cancel(item.id);
    chrome.downloads.erase({ id: item.id });
    await sendToBits({
      url: item.url,
      referer: item.referrer || "",
      userAgent: navigator.userAgent
    });
  } catch {
    // ignore
  }
});

async function sendToBits(payload) {
  try {
    const res = await fetch(`${BITS_BRIDGE_URL}/add`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify(payload)
    });

    if (res.ok) {
      chrome.notifications?.create({
        type: "basic",
        iconUrl: "data:image/svg+xml;charset=utf-8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='%234f8cff'><path d='M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M7 10l5 5 5-5M12 15V3'/></svg>",
        title: "Bits Download Manager",
        message: "Download added successfully!"
      });
    }
  } catch (err) {
    console.warn("Failed to send download to Bits Download Manager:", err);
  }
}
