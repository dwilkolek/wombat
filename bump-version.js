import * as fs from 'node:fs';

const newVersion = `${process.argv[2]}`;
const safeNewVersion = newVersion.match(/([0-9]+.[0-9]+.[0-9]+)/)[0];

console.log(`New version ${newVersion}(${safeNewVersion})`);

const packageJsonPath = './package.json';
const packageJson = JSON.parse(fs.readFileSync(packageJsonPath).toString('utf8'));
packageJson['version'] = safeNewVersion;
fs.writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, '\t'));

const tauriConfPath = './src-tauri/tauri.conf.json';
const tauriConfJson = JSON.parse(fs.readFileSync(tauriConfPath).toString('utf8'));
tauriConfJson['version'] = safeNewVersion;
fs.writeFileSync(tauriConfPath, JSON.stringify(tauriConfJson, null, '\t'));

const cargoTomlPath = './src-tauri/Cargo.toml';
const cargoToml = fs.readFileSync(cargoTomlPath).toString('utf8');
const start = cargoToml.search(/version = "([0-9.]+)"/gm);
const end = cargoToml.indexOf('\n', start);
fs.writeFileSync(
	cargoTomlPath,
	cargoToml.substring(0, start) + `version = "${safeNewVersion}"` + cargoToml.substring(end)
);
