/* global */
/** @type {Array.<Cookie>} */
let cookies = [];
const extVersion = '4.0.2'
let syncEnabled = false;
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

/**
    @param {String} name
    @param {String} env
    @returns {String}
*/
function storageCookieKey(name, env) {
	return `v2-${name}-${env}`;
}

chrome.storage.local.get(
	cookiesConfig.map((e) => storageCookieKey(e.name, e.env)),
	(v) => {
		console.log('restored cookies', v);
		Object.entries(v).forEach((cv) => {
			cookies.push(cv[1]);
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
	popupOpen &&
		chrome.runtime.sendMessage({
			action: 'desktopApp',
			wombatVersion: wombatOpen
		});
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
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
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
setInterval(async () => {
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
		.catch((e) => {
			console.warn(e);
			return undefined;
		});
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

			chrome.storage.local.set({ [storageCookieKey(cookie.name, cookie.env)]: cookie });
			console.log('updating session key', cookie);
			sendCookieToDesktop(cookie);
			sendCookieToPopup(cookie);
		});
	});
}, 1000);

setInterval(() => {
	popupOpen &&
		chrome.runtime.sendMessage({
			action: 'extVersion',
			extVersion: extVersion
		});
});
