function execute() {
	const continueBtn = document.querySelector('button[type=submit]');
	if (continueBtn) {
		continueBtn.click();
		clearInterval(interval);
	}
}
let interval = setInterval(execute, 1000);
