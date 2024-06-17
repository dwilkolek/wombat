function execute(i) {
	if (document.querySelector('h1').innerText == 'Single sign-on to TechnipFMC - EMU') {
		const continueBtn = document.querySelector('button[type=submit]');
		if (continueBtn) {
			continueBtn.click();
		}
	}
	if (i < 10) {
		setTimeout(() => execute(i + 1), 200);
	}
}
setTimeout(() => execute(0));
