function execute(i) {
	const allowBtn = document.querySelector('button[data-testid=allow-access-button]');
	if (allowBtn) {
		console.log('allow button found', allowBtn);
		window.umami.trackSafe('aws-step-2');
		allowBtn.click();
		setInterval(() => {
			if (document.querySelector('.awsui-context-alert')?.innerText?.includes('Request approved')) {
				chrome.runtime.sendMessage({ action: 'closeTab' });
			}
		}, 400);
		return;
	}
	if (i < 10) {
		setTimeout(() => execute(i + 1), 200);
	}
}

setTimeout(() => execute(0));
