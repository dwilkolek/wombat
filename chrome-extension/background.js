/* global */
const cookies = {};
let syncEnabled = false;
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
chrome.storage.local.get(
	cookiesConfig.map((e) => e.name),
	(v) => {
		console.log('restored cookies', v);
		Object.entries(v).forEach((cv) => {
			cookies[cv[0]] = cv[1];
		});
		syncEnabled = true;
	}
);

function now() {
	return new Date().getTime();
}
function isCookieFresh(dateTimeMillis) {
	return now() - dateTimeMillis < 5 * 60 * 1000;
}

/* popup */
let popupOpen = false;
function sendCookieToPopup(name, value, storedAt) {
	if (popupOpen) {
		chrome.runtime.sendMessage({
			action: 'newCookie',
			name,
			cookie: isCookieFresh(storedAt) ? value ?? '<null>' : '<null>'
		});
	}
}

function notifyPopupDeskopCLientOnline() {
	popupOpen && chrome.runtime.sendMessage({ action: 'desktopApp', alive: wombatOpen });
}

/* desktop */
let wombatOpen = false;
function sendCookieToDesktop(name, value, storedAt) {
	if (!isCookieFresh(storedAt)) {
		return;
	}
	if (value) {
		fetch(`http://localhost:6891/cookies/${name}`, {
			body: JSON.stringify(value),
			method: 'PUT',
			headers: { Accept: 'application/json', 'Content-Type': 'application/json' }
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
}
function notifyDeskopClient() {
	Object.entries(cookies).forEach((cookie) => {
		sendCookieToDesktop(cookie[0], cookie[1][0], cookie[1][1]);
	});
}

/* event listeners */
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
	if (request.action === 'closeTab') {
		chrome.tabs.remove(sender.tab.id);
	}
});

chrome.runtime.onConnect.addListener(function (port) {
	if (port.name === 'popup') {
		popupOpen = true;
		chrome.runtime.sendMessage({ action: 'desktopApp', alive: wombatOpen });
		Object.entries(cookies).forEach((entry) => {
			sendCookieToPopup(entry[0], entry[1][0], entry[1][1]);
		});
		port.onDisconnect.addListener(function () {
			popupOpen = false;
		});
	}
});

/* popup opened/closed */
setInterval(async () => {
	const prevWombatOpen = wombatOpen;
	wombatOpen = await fetch(`http://localhost:6891/health`)
		.then(() => true)
		.catch(() => false);
	if (prevWombatOpen !== wombatOpen && wombatOpen) {
		notifyDeskopClient();
		notifyPopupDeskopCLientOnline();
	}
}, 1000);

/* checking stored session cookies */
setInterval(function () {
	if (!syncEnabled) {
		return;
	}
	cookiesConfig.forEach(({ name, domain }) => {
		chrome.cookies.get({ url: domain, name: name }, (cookie) => {
			const cookieValue = cookie?.value;
			const [oldCookieValue] = cookies[name] ?? [null, now()];
			if (oldCookieValue == cookieValue) {
				return;
			}
			cookies[name] = [cookieValue, now()];
			chrome.storage.local.set({ [name]: [cookieValue, now()] });
			console.log('updating session key', name);
			sendCookieToDesktop(name, cookieValue, now());
			sendCookieToPopup(name, cookieValue, now());
		});
	});
}, 1000);
