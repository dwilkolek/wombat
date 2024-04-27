const express = require('express');
const app = express();
const port = 3000;

app.get('/', (req, res) => {
	res.send('Release server is running!');
});

app.use(express.static('static'));
app.listen(port, () => {
	console.log(`Release server app listening on port ${port}`);
});
