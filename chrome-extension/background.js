console.log('background.js loaded');
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
	if (request.action === 'closeTab') {
		chrome.tabs.remove(sender.tab.id);
	}
});
let popupOpen = false;
const cookies = {};
chrome.runtime.onConnect.addListener(function (port) {
	if (port.name === 'popup') {
		popupOpen = true;
		chrome.runtime.sendMessage({ action: 'desktopApp', alive: wombatOpen });
		Object.entries(cookies).forEach((entry) => {
			console.log('Sending ', entry);
			chrome.runtime.sendMessage({
				action: 'newCookie',
				name: entry[0],
				cookie: entry[1] ?? '<null>'
			});
		});
		port.onDisconnect.addListener(function () {
			console.log('popup has been closed');
			popupOpen = false;
		});
	}
});

const cookiesConfig = [
	{
		name: 'session-v1-dev',
		domain: 'https://dxp.dev.services.technipfmc.com'
	},
	{
		name: 'session-v1-demo',
		domain: 'https://dxp.demo.services.technipfmc.com'
	},
	{
		name: 'session-v1-prod',
		domain: 'https://dxp.services.technipfmc.com'
	}
];

let wombatOpen = false;
setInterval(async () => {
	try {
		wombatOpen = await fetch(`http://localhost:6891/ping`)
			.then(() => true)
			.catch(() => false);

		popupOpen && chrome.runtime.sendMessage({ action: 'desktopApp', alive: wombatOpen });
	} catch (e) {}
}, 1000);

setInterval(function () {
	cookiesConfig.forEach(({ name, domain }) => {
		console.log(`fetching cookie for ${domain} and ${name}`);
		chrome.cookies.get({ url: domain, name: name }, (cookie) => {
			const cookieValue = cookie?.value;
			console.log('cookie value', cookie, cookies[name], cookieValue);
			if (cookies[name] === cookieValue) {
				return;
			}
			cookies[name] = cookieValue;

			if (cookieValue) {
				fetch(`http://localhost:6891/cookies/${name}`, {
					body: cookie.value,
					method: 'PUT'
				})
					.then(() => {})
					.catch(() => {});
			} else {
				fetch(`http://localhost:6891/cookies/${name}`, {
					method: 'DELETE'
				})
					.then(() => {})
					.catch(() => {});
			}
			popupOpen &&
				chrome.runtime.sendMessage({ action: 'newCookie', name, cookie: cookieValue ?? '<null>' });
		});
	});
}, 1000);

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
