# Wombat

## Wombat presents view at AWS services. I created it to learn Tauri and Svelte... and make my life easier 😎

_This project is no longer generic tool to use AWS._

## Features

- Displaying ECS services across clusters with assosiated RDS instance
- Proxing to ECS/RDS instance
- Proxing to ECS can be configured to inject custom headers, authorize with jepsen/basic auth
- Searching through cloudwatch logs
- Finding password to RDS

## Quirks

- DBs that have the password stored in Secrets Manager result in a temporary connection in Dbeaver
- DBs that have the password stored in SSM result in a permanent connection in Dbeaver (probably you'll need to fix the user/database name)
- After you run the app on MacOS you need to trust it in the "Privacy and Security" tab

## Requirements

- [AWS CLI](https://aws.amazon.com/cli/)
- [Session manager plugin](https://docs.aws.amazon.com/systems-manager/latest/userguide/session-manager-working-with-install-plugin.html)
- Configured aws cli

## Expected aws configuration

### Single SSO profile with infra profile per app

```
[profile <profile>]
sso_start_url =
sso_region =
sso_account_id =
sso_role_name =
region =

[profile <service1>]
role_arn = arn:aws:iam::123567890:role/<service1>-infra
source_profile = <profile>

[profile <service2>]
role_arn = arn:aws:iam::123567890:role/<service2>-infra
source_profile = <profile>
```

### Single SSO profile with infra profile per app per dev

```
[profile <profile>]
sso_start_url =
sso_region =
sso_account_id =
sso_role_name =
region =

[profile <service1>-dev]
role_arn = arn:aws:iam::123567890:role/<service1>-infra
source_profile = <profile>

[profile <service2>-dev]
role_arn = arn:aws:iam::123567890:role/<service2>-infra
source_profile = <profile>

[profile <service2>-demo]
role_arn = arn:aws:iam::123567890:role/<service2>-infra
source_profile = <profile>
```

### SSO profile per environment with infra profile per app per dev

_Use this for multiple AWS accounts_

```
[profile <profile>-dev]
sso_start_url =
sso_region =
sso_account_id =
sso_role_name =
region =

[profile <service1>-dev]
role_arn = arn:aws:iam::123567890:role/<service1>-infra
source_profile = <profile>-dev

[profile <service2>-dev]
role_arn = arn:aws:iam::123567890:role/<service2>-infra
source_profile = <profile>-dev

[profile <profile>-demo]
sso_start_url =
sso_region =
sso_account_id =
sso_role_name =
region =

[profile <service1>-demo]
role_arn = arn:aws:iam::123567890:role/<service1>-infra
source_profile = <profile>-demo

[profile <service2>-demo]
role_arn = arn:aws:iam::123567890:role/<service2>-infra
source_profile = <profile>-demo

```

#### Bump version

`npm run bump-version x.x.x`

#### Publish chrome extension

Zip contets of `chrome-extension` and upload in chrome web store dev console.

Remember to check `requirements.rs`, `manifest.json` and `background.js`.
