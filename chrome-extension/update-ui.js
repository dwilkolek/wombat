(function() {
  chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
    if (request.action === "newCookie") {
      document.getElementById(request.name).innerText = request.cookie ?? 'NONE';
    }
  })
})();
