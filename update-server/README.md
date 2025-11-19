# Update server

1. Clone selected release

```bash
cd static
gh release download <tag>
```

2. Update local tauri.conf.json to `"http://localhost:52137/latest.json"`
3. Start server

```bash
cd static
pnpm start
```
