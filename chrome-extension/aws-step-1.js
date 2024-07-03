(function () {
	const verifyBtn = document.querySelector('#cli_verification_btn');
	if (verifyBtn) {
		console.log('verify button found', verifyBtn);
		window.umami.trackSafe('aws-step-1');
		verifyBtn.click();
	}
})();
