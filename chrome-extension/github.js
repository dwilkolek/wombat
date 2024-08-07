async function execute(i) {
	const header = document.querySelector('h1');
	if (header && header.innerText == 'Single sign-on to TechnipFMC - EMU') {
		const continueBtn = document.querySelector('button[type=submit]');
		if (continueBtn) {
			await window.umami.trackSafe('github_auth');
			continueBtn.click();
			return;
		}
	}
	if (i < 10) {
		setTimeout(() => execute(i + 1), 200);
	}
}
setTimeout(() => execute(0));
