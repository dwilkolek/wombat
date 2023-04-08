# Wombat

A Portal to AWS created to checkout Tauri and Svelte.

## Features

- Proxy to RDS
- Proxy to ECS (injected Origin/Host headers)
- All proxies have permanently assigned port
- Tracking of favorite services
- Each service is tracked separately - it requires you to manually favorite the same service per environment

## Quirks

- DBs that have the password stored in Secrets Manager result in a temporary connection in Dbeaver
- DBs that have the password stored in SSM result in a permanent connection in Dbeaver (probably you'll need to fix the user/database name)
- After you run the app on MacOS you need to trust it in the "Privacy and Security" tab

## Issues

- Migration after immutable deployment
