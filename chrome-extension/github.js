function execute() {
	if (document.querySelector('h1').innerText == 'Single sign-on to TechnipFMC - EMU') {
		const continueBtn = document.querySelector('button[type=submit]');
		if (continueBtn) {
			continueBtn.click();
			clearInterval(interval);
		}
	}
}
let interval = setInterval(execute, 1000);
