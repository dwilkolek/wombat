import execa from 'execa';
import * as fs from 'node:fs';

let extension = '';
let targetExtension = '';
if (process.platform === 'win32') {
	targetExtension = '.exe';
}

async function main() {
	const rustInfo = (await execa('rustc', ['-vV'])).stdout;
	const targetTriple = /host: (\S+)/g.exec(rustInfo)[1];
	if (!targetTriple) {
		console.error('Failed to determine platform target triple');
	}
	fs.renameSync(
		`src-tauri/binaries/arh${extension}`,
		`src-tauri/binaries/arh-${targetTriple}${targetExtension}`
	);
}

main().catch((e) => {
	throw e;
});
