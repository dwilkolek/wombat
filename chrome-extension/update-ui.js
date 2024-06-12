(function() {
  chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
    if (request.action === "newCookie") {
      document.getElementById(request.name).innerText = request.cookie ?? 'NONE';
    }

    if (request.action === "desktopApp") {
      document.getElementById('desktopAppHealth').innerText = request.alive ? 'RUNNING' : 'CLOSED';
    }
  })
})();
chrome.runtime.connect({ name: "popup" });
