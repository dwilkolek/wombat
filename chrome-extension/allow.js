function execute() {
  const allowBtn = document.querySelector("button[data-testid=allow-access-button]")
  if (allowBtn) {
    console.log("allow button found", allowBtn)
    interval && clearInterval(interval)
    allowBtn.click();
    setInterval(() => {
      if (document.querySelector(".awsui-context-alert")?.innerText?.includes("Request approved")) {
        chrome.runtime.sendMessage({ action: "closeTab" });
      }
    }, 400)
    return;
  }
}
let interval = setInterval(execute, 1000)
console.log("allow.js loaded");
