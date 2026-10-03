const BITS_BRIDGE_URL = "http://127.0.0.1:6801";

document.addEventListener("DOMContentLoaded", async () => {
  const statusDot = document.getElementById("status-dot");
  const toggle = document.getElementById("intercept-toggle");
  const quickUrl = document.getElementById("quick-url");
  const addBtn = document.getElementById("add-btn");
  const feedback = document.getElementById("feedback");

  // Load saved interception preference
  const saved = await chrome.storage.local.get({ intercept: false });
  toggle.checked = saved.intercept;

  toggle.addEventListener("change", () => {
    chrome.storage.local.set({ intercept: toggle.checked });
  });

  // Check desktop app connection
  try {
    const res = await fetch(`${BITS_BRIDGE_URL}/ping`, { method: "GET" });
    if (res.ok) {
      statusDot.className = "status-dot online";
      statusDot.title = "Connected to Bits Download Manager";
    } else {
      statusDot.className = "status-dot offline";
      statusDot.title = "Bits Download Manager not reachable";
    }
  } catch {
    statusDot.className = "status-dot offline";
    statusDot.title = "Bits Download Manager is not running";
  }

  // Quick download submit
  addBtn.addEventListener("click", async () => {
    const url = quickUrl.value.trim();
    if (!url) return;

    try {
      const res = await fetch(`${BITS_BRIDGE_URL}/add`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ url })
      });

      if (res.ok) {
        feedback.className = "feedback success";
        feedback.textContent = "Added to download queue!";
        quickUrl.value = "";
      } else {
        feedback.className = "feedback error";
        feedback.textContent = "Failed to add download.";
      }
    } catch {
      feedback.className = "feedback error";
      feedback.textContent = "Cannot connect to Bits Desktop app.";
    }

    setTimeout(() => {
      feedback.textContent = "";
    }, 3000);
  });
});
