import * as fs from 'node:fs';
import * as cp from 'child_process';

const newVersion = `${process.argv[2]}`;
const safeNewVersion = newVersion.match(/([0-9]+.[0-9]+.[0-9]+)/)[0]

console.log(`New version ${newVersion}(${safeNewVersion})`);

const packageJsonPath = './package.json';
const packageJson = JSON.parse(fs.readFileSync(packageJsonPath).toString('utf8'));
packageJson['version'] = safeNewVersion;
fs.writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 4));

const tauriConfPath = './src-tauri/tauri.conf.json';
const tauriConfJson = JSON.parse(fs.readFileSync(tauriConfPath).toString('utf8'));
tauriConfJson['package']['version'] = newVersion;
fs.writeFileSync(tauriConfPath, JSON.stringify(tauriConfJson, null, 2));

const cargoTomlPath = './src-tauri/Cargo.toml';
const cargoToml = fs.readFileSync(cargoTomlPath).toString('utf8');
const start = cargoToml.search(/version = "([0-9.]+)"/gm);
const end = cargoToml.indexOf('\n', start);
fs.writeFileSync(
	cargoTomlPath,
	cargoToml.substring(0, start) + `version = "${safeNewVersion}"` + cargoToml.substring(end)
);

cp.execSync('npm install');
cp.execSync('cd src-tauri && cargo generate-lockfile && cd ..');
cp.execSync(`git commit -a -m"Release v${newVersion}"`);
cp.execSync(`git tag v${newVersion}`);
cp.execSync(`git push origin v${newVersion}`);

console.log(`Done`);
