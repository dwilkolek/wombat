# Wombat

### Wombat presents view at AWS services. I created it to learn Tauri and Svelte... and make my life easier 😎

![Wombat home](https://github.com/dwilkolek/wombat/blob/main/docs/wombat-home.png?raw=true)
![Wombat logs](https://github.com/dwilkolek/wombat/blob/main/docs/wombat-logs.png?raw=true)
## Features

- Proxy to RDS
- Proxy to ECS (injected Origin/Host headers)
- Searching through cloudwatch logs
- All proxies have permanently assigned port
- Tracking of favorite services
- Each service is tracked separately - it requires you to manually favorite the same service per environment
- Jepsen proxy

## Quirks

- DBs that have the password stored in Secrets Manager result in a temporary connection in Dbeaver
- DBs that have the password stored in SSM result in a permanent connection in Dbeaver (probably you'll need to fix the user/database name)
- After you run the app on MacOS you need to trust it in the "Privacy and Security" tab



## Bump version

`npm run bump-version x.x.x`