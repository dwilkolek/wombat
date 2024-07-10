async function execute(i) {
	if (
		document.body.innerHTML ==
		'Your identity was confirmed and propagated to Snowflake JDBC driver. You can close this window now and go back where you started from.'
	) {
		await window.umami.trackSafe('snowflake-auth');
		chrome.runtime.sendMessage({ action: 'closeTab' });
		return;
	}
	if (i < 10) {
		setTimeout(() => execute(i + 1), 200);
	}
}

setTimeout(async () => execute(0));
