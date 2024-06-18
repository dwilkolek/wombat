function execute(i) {
	if (
		document.body.innerHTML ==
		'Your identity was confirmed and propagated to Snowflake JDBC driver. You can close this window now and go back where you started from.'
	) {
		chrome.runtime.sendMessage({ action: 'trackedEvent', event: 'snowflake-auth' });
		chrome.runtime.sendMessage({ action: 'closeTab' });
	}
	if (i < 10) {
		setTimeout(() => execute(i + 1), 200);
	}
}

setTimeout(() => execute(0));
