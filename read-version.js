import * as fs from 'node:fs';
const packageJsonPath = './package.json';
const packageJson = JSON.parse(fs.readFileSync(packageJsonPath).toString('utf8'));
console.log(packageJson['version']);
