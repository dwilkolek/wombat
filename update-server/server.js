import express from 'express';
import fs from 'node:fs';
const app = express();
const port = 52137;

app.use(express.static('static'));
const latestJsonPath = 'static/latest.json';
const f = JSON.parse(fs.readFileSync(latestJsonPath));

for (const platform in f.platforms) {
	const originalUrl = f.platforms[platform].url;
	f.platforms[platform].url = 'http://localhost:' + port + '/' + originalUrl.split('/').at(-1);
	console.log(originalUrl, f.platforms[platform].url);
}

fs.writeFileSync(latestJsonPath, JSON.stringify(f, null, 2));

app.listen(port, () => {
	console.log(`Update server listening on port ${port}`);
});
