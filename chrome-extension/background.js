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
			chrome.runtime.sendMessage({
				action: 'newCookie',
				name: entry[0],
				cookie: entry[1] ?? '<null>'
			});
		});
		port.onDisconnect.addListener(function () {
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
		const prevWombatOpen = wombatOpen;
		wombatOpen = await fetch(`http://localhost:6891/health`)
			.then(() => true)
			.catch(() => false);
		if (prevWombatOpen !== wombatOpen && wombatOpen) {
			Object.entries(cookies).forEach((entry) => {
				const name = entry[0];
				const value = entry[1];
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
			});
		}
		popupOpen && chrome.runtime.sendMessage({ action: 'desktopApp', alive: wombatOpen });
	} catch (e) {}
}, 1000);

setInterval(function () {
	cookiesConfig.forEach(({ name, domain }) => {
		chrome.cookies.get({ url: domain, name: name }, (cookie) => {
			const cookieValue = cookie?.value;
			if (cookies[name] === cookieValue) {
				return;
			}
			cookies[name] = cookieValue;

			if (cookieValue) {
				fetch(`http://localhost:6891/cookies/${name}`, {
					body: JSON.stringify(cookie.value),
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
			popupOpen &&
				chrome.runtime.sendMessage({ action: 'newCookie', name, cookie: cookieValue ?? '<null>' });
		});
	});
}, 1000);
