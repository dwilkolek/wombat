// const umamiScript = document.createElement('script');
// umamiScript.setAttribute('src', 'https://umami.wilkolek.eu/script.js');
// umamiScript.setAttribute('data-website-id', 'dc4bbfa3-79fe-4f04-bd34-92a55956847e');
// document.head.appendChild(umamiScript);
((window) => {
	const {
		screen: { width, height },
		navigator: { language },
		location,
		localStorage,
		document,
		history
	} = window;
	const { hostname, href } = location;
	const { referrer } = document;

	const _data = 'data-';
	const website = 'dc4bbfa3-79fe-4f04-bd34-92a55956847e';
	const hostUrl = 'https://umami.wilkolek.eu';
	const tag = undefined;
	const autoTrack = false;
	const excludeSearch = false;
	const domain = '';
	const domains = domain.split(',').map((n) => n.trim());
	const host = hostUrl;
	const endpoint = `${host.replace(/\/$/, '')}/api/send`;
	const screen = `${width}x${height}`;
	const eventRegex = /data-umami-event-([\w-_]+)/;
	const eventNameAttribute = _data + 'umami-event';
	const delayDuration = 300;

	/* Helper functions */

	const encode = (str) => {
		if (!str) {
			return undefined;
		}

		try {
			const result = decodeURI(str);

			if (result !== str) {
				return result;
			}
		} catch {
			return str;
		}

		return encodeURI(str);
	};

	const parseURL = (url) => {
		try {
			const { pathname, search } = new URL(url);
			url = pathname + search;
		} catch {
			/* empty */
		}
		return excludeSearch ? url.split('?')[0] : url;
	};

	const getPayload = () => ({
		website,
		hostname,
		screen,
		language,
		title: encode(title),
		url: encode(currentUrl),
		referrer: encode(currentRef),
		tag: tag ? tag : undefined
	});

	/* Event handlers */

	const handlePush = (state, title, url) => {
		if (!url) return;

		currentRef = currentUrl;
		currentUrl = parseURL(url.toString());

		if (currentUrl !== currentRef) {
			setTimeout(track, delayDuration);
		}
	};

	const handlePathChanges = () => {
		const hook = (_this, method, callback) => {
			const orig = _this[method];

			return (...args) => {
				callback.apply(null, args);

				return orig.apply(_this, args);
			};
		};

		history.pushState = hook(history, 'pushState', handlePush);
		history.replaceState = hook(history, 'replaceState', handlePush);
	};

	const handleTitleChanges = () => {
		const observer = new MutationObserver(([entry]) => {
			title = entry && entry.target ? entry.target.text : undefined;
		});

		const node = document.querySelector('head > title');

		if (node) {
			observer.observe(node, {
				subtree: true,
				characterData: true,
				childList: true
			});
		}
	};

	const handleClicks = () => {
		document.addEventListener(
			'click',
			async (e) => {
				const isSpecialTag = (tagName) => ['BUTTON', 'A'].includes(tagName);

				const trackElement = async (el) => {
					const attr = el.getAttribute.bind(el);
					const eventName = attr(eventNameAttribute);

					if (eventName) {
						const eventData = {};

						el.getAttributeNames().forEach((name) => {
							const match = name.match(eventRegex);

							if (match) {
								eventData[match[1]] = attr(name);
							}
						});

						return track(eventName, eventData);
					}
				};

				const findParentTag = (rootElem, maxSearchDepth) => {
					let currentElement = rootElem;
					for (let i = 0; i < maxSearchDepth; i++) {
						if (isSpecialTag(currentElement.tagName)) {
							return currentElement;
						}
						currentElement = currentElement.parentElement;
						if (!currentElement) {
							return null;
						}
					}
				};

				const el = e.target;
				const parentElement = isSpecialTag(el.tagName) ? el : findParentTag(el, 10);

				if (parentElement) {
					const { href, target } = parentElement;
					const eventName = parentElement.getAttribute(eventNameAttribute);

					if (eventName) {
						if (parentElement.tagName === 'A') {
							const external =
								target === '_blank' ||
								e.ctrlKey ||
								e.shiftKey ||
								e.metaKey ||
								(e.button && e.button === 1);

							if (eventName && href) {
								if (!external) {
									e.preventDefault();
								}
								return trackElement(parentElement).then(() => {
									if (!external) location.href = href;
								});
							}
						} else if (parentElement.tagName === 'BUTTON') {
							return trackElement(parentElement);
						}
					}
				} else {
					return trackElement(el);
				}
			},
			true
		);
	};

	/* Tracking functions */

	const trackingDisabled = () =>
		!website ||
		(localStorage && localStorage.getItem('umami.disabled')) ||
		(domain && !domains.includes(hostname));

	const send = async (payload, type = 'event') => {
		if (trackingDisabled()) return;

		const headers = {
			'Content-Type': 'application/json'
		};

		if (typeof cache !== 'undefined') {
			headers['x-umami-cache'] = cache;
		}

		try {
			const res = await fetch(endpoint, {
				method: 'POST',
				body: JSON.stringify({ type, payload }),
				headers
			});
			const text = await res.text();

			return (cache = text);
		} catch {
			/* empty */
		}
	};

	const track = (obj, data) => {
		if (typeof obj === 'string') {
			return send({
				...getPayload(),
				name: obj,
				data: typeof data === 'object' ? data : undefined
			});
		} else if (typeof obj === 'object') {
			return send(obj);
		} else if (typeof obj === 'function') {
			return send(obj(getPayload()));
		}
		return send(getPayload());
	};

	const identify = (data) => send({ ...getPayload(), data }, 'identify');

	/* Start */

	if (!window.umami) {
		window.umami = {
			track,
			identify,
			trackSafe: async (obj, data) => {
				try {
					await track(obj, data);
				} catch (e) {
					console.warn('Error sending', e);
				}
			}
		};
	}

	let currentUrl = parseURL(href);
	let currentRef = referrer !== hostname ? referrer : '';
	let title = document.title;
	let cache;
	let initialized;

	if (autoTrack && !trackingDisabled()) {
		handlePathChanges();
		handleTitleChanges();
		handleClicks();

		const init = () => {
			if (document.readyState === 'complete' && !initialized) {
				track();
				initialized = true;
			}
		};

		document.addEventListener('readystatechange', init, true);

		init();
	}
})(window);
