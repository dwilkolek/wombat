async function execute(i) {
	const allowBtn = document.querySelector('button[data-testid=allow-access-button]');
	if (allowBtn) {
		console.log('allow button found', allowBtn);
		await window.umami.trackSafe('aws_auth');
		allowBtn.click();
		let interval = setInterval(() => {
			if (document.querySelector('.awsui-context-alert')?.innerText?.includes('Request approved')) {
				clearInterval(interval);
				chrome.runtime.sendMessage({ action: 'closeTab' });
				return;
			}
		}, 400);

		return;
	}
	if (i < 10) {
		setTimeout(async () => execute(i + 1), 200);
	}
}

setTimeout(() => execute(0));
