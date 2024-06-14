(function () {
	chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
		if (request.action === 'newCookie') {
			console.log(request);
			const cookie = request.cookie;
			const cookieValueId = `cookie-${cookie.name}-${cookie.env}`;
			const el = document.getElementById(cookieValueId);
			if (el) {
				el.innerText = cookie.value ?? '<decayed>';
			} else {
				const span = document.createElement('span');
				span.id = cookieValueId;
				span.innerText = cookie.value;

				const p = document.createElement('p');
				p.append(cookie.name, '=', span);
				document.getElementById('cookies').append(p);
			}
		}

		if (request.action === 'desktopApp') {
			document.getElementById('desktopAppHealth').innerText = request.alive ? 'RUNNING' : 'CLOSED';
		}
	});
})();
chrome.runtime.connect({ name: 'popup' });
