console.log("background.js loaded");
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  if (request.action === "closeTab") {
    chrome.tabs.remove(sender.tab.id);
  }
  if (request.action === "getSessionCookies") {
    getCookie()
  }
});
const cookies = [
  {
    name: "session-v1-dev",
    domain: "https://dxp.dev.services.technipfmc.com"
  },
  {
    name: "session-v1-demo",
    domain: "https://dxp.demo.services.technipfmc.com"
  },
  {
    name: "session-v1-prod",
    domain: "https://dxp.services.technipfmc.com"
  }
];
setInterval(getCookie, 1000);
function getCookie() {
  cookies.forEach(({ name, domain }) => {
    console.log(`fetching cookie for ${domain} and ${name}`)
    chrome.cookies.get({ url: domain, name: name }, (cookie) => {
      if (cookie) {
        console.log(`Cookie value: ${cookie.value}`);
        chrome.runtime.sendMessage({ action: "newCookie", name, cookie: cookie.value });
      } else {
        console.log('Cookie not found');
        chrome.runtime.sendMessage({ action: "newCookie", name, cookie: '<null>' });
      }
    })
  })
}

// chrome.webRequest.onCompleted.addListener(
//   function(details) {
//     console.log('Request completed:', details.url, details);
//     details.responseHeaders?.some(function(header) {
//       if (header.name.includes('Cookie')) {
//         console.log("New Cookie value:" + header.value);
//         // return true;
//       }
//       // return false;
//     });
//   },
//   {
//     urls: ["*://*.services.technipfmc.com/*"],
//     // urls: ['*://*.services.technipfmc.com/*', '*://*.dev.services.technipfmc.com/*'],
//     types: ['xmlhttprequest']
//   },
//   ["responseHeaders", "extraHeaders"]
// );



// chrome.webRequest.onBeforeSendHeaders.addListener(
//   //
//   // details.requestHeaders // check me out
//   details => console.log(details.url, details.requestHeaders),
//   { urls: ["*://*.services.technipfmc.com/*"] },
//   ["requestHeaders"]);

// chrome.webRequest.onHeadersReceived.addListener(
//   function(details) {
//     console.log('Request headers received:', details);
//   },
//   {
//     urls: ["*://*.dev.services.technipfmc.com/*"],
//     // urls: ['*://*.services.technipfmc.com/*', '*://*.dev.services.technipfmc.com/*'],
//     types: ['xmlhttprequest']
//   },
//   ["responseHeaders"]
// );
// chrome.webRequest.onErrorOccurred.addListener(
//   function(details) {
//     console.log('Request headers err:', details);
//   },
//   {
//     urls: ["*://*.dev.services.technipfmc.com/*"],
//     // urls: ['*://*.services.technipfmc.com/*', '*://*.dev.services.technipfmc.com/*'],
//     types: ['xmlhttprequest']
//   }
// );

// async function closeTab() {
//   let queryOptions = { active: true, lastFocusedWindow: true };
//   // `tab` will either be a `tabs.Tab` instance or `undefined`.
//   let [tab] = await chrome.tabs.query(queryOptions);

//   if (tab) {
//     chrome.tabs.remove(
//       tab,
//     )
//   }
// }
