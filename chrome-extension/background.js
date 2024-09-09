/* global */
/** @type {Array.<Cookie>} */
let cookies = [];
const extVersion = '5.0.2';
let syncEnabled = true;
class Cookie {
	constructor(name, value, env) {
		this.name = name;
		this.value = value;
		this.env = env;
		this.storedAt = now();
	}
}
const cookiesConfig = [
	{
		name: 'session-v1-dev',
		domain: 'https://dxp.dev.services.technipfmc.com',
		env: 'DEV'
	},
	{
		name: 'session-v1-demo',
		domain: 'https://dxp.demo.services.technipfmc.com',
		env: 'DEMO'
	},
	{
		name: 'session-v1',
		domain: 'https://dxp.services.technipfmc.com',
		env: 'PROD'
	}
];

function now() {
	return new Date().getTime();
}
function isCookieFresh(dateTimeMillis) {
	return now() - dateTimeMillis < 5 * 60 * 1000;
}

/* popup */
let popupOpen = false;
/** @param {Cookie} cookie */
function sendCookieToPopup(cookie) {
	if (popupOpen) {
		chrome.runtime.sendMessage({
			action: 'newCookie',
			cookie: cookie
		});
	}
}

function notifyPopupDeskopCLientOnline() {
	if (popupOpen) {
		chrome.runtime.sendMessage({
			action: 'desktopApp',
			wombatVersion: wombatOpen
		});
	}
}

/* desktop */
/** @type {String|undefined} */
let wombatOpen = undefined;
/** @param {Cookie} cookie */
function sendCookieToDesktop(cookie) {
	if (!isCookieFresh(cookie.storedAt)) {
		return;
	}
	if (cookie.value) {
		fetch(`http://localhost:6891/cookies`, {
			body: JSON.stringify(cookie),
			method: 'PUT',
			headers: { Accept: 'application/json', 'Content-Type': 'application/json' }
		})
			.then(() => {})
			.catch(() => {});
	} else {
		fetch(`http://localhost:6891/cookies/${cookie.name}`, {
			method: 'DELETE'
		})
			.then(() => {})
			.catch(() => {});
	}
}

function notifyDeskopClient() {
	cookies.forEach((cookie) => {
		sendCookieToDesktop(cookie);
	});
}

/* event listeners */
chrome.runtime.onMessage.addListener((request, sender) => {
	if (request.action === 'closeTab') {
		chrome.tabs.remove(sender.tab.id);
	}
});

chrome.runtime.onMessage.addListener((request, sender) => {
	if (request.action === 'closeTab') {
		chrome.tabs.remove(sender.tab.id);
	}
});

chrome.runtime.onConnect.addListener(function (port) {
	if (port.name === 'popup') {
		popupOpen = true;
		chrome.runtime.sendMessage({ action: 'desktopApp', wombatVersion: wombatOpen });
		cookies.forEach((cookie) => {
			sendCookieToPopup(cookie);
		});
		port.onDisconnect.addListener(function () {
			popupOpen = false;
		});
	}
});

/* popup opened/closed */
let healthTimeout;
const healthCheck = async () => {
	clearTimeout(healthTimeout);
	const prevWombatOpen = wombatOpen;
	wombatOpen = await fetch(`http://localhost:6891/health`, {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json'
		},
		body: JSON.stringify(extVersion)
	})
		.then((resp) => {
			console.log(resp);
			return resp.text();
		})
		.catch(() => {
			// console.warn(e);
			return undefined;
		});

	if (prevWombatOpen !== wombatOpen && wombatOpen) {
		notifyDeskopClient();
		notifyPopupDeskopCLientOnline();
	}
	healthTimeout = setTimeout(healthCheck, 1000);
};
healthCheck();

/* checking stored session cookies */
setInterval(function () {
	if (!syncEnabled) {
		return;
	}
	cookiesConfig.forEach(({ name, domain, env }) => {
		chrome.cookies.get({ url: domain, name: name }, (chromeCookie) => {
			const cookieValue = chromeCookie?.value;
			const oldCookie = cookies.find((c) => c.name == name && c.env == env);
			if (oldCookie?.value == cookieValue) {
				return;
			}
			const cookie = new Cookie(name, cookieValue, env);

			cookies = cookies.filter((c) => c.name != name || c.env != env);
			cookies.push(cookie);

			console.log('updating session key', cookie);
			sendCookieToDesktop(cookie);
			sendCookieToPopup(cookie);
		});
	});
}, 1000);

setInterval(() => {
	if (popupOpen) {
		chrome.runtime.sendMessage({
			action: 'extVersion',
			extVersion: extVersion
		});
	}
}, 1000);
