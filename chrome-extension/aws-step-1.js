function execute() {
  const verifyBtn = document.querySelector("#cli_verification_btn");
  if (verifyBtn) {
    console.log("verify button found", verifyBtn)
    verifyBtn.click();
    clearInterval(interval)
  }

}
let interval = setInterval(execute, 1000)
