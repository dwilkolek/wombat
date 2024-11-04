<script lang="ts">
	import { run, preventDefault } from 'svelte/legacy';

	import { ENVIRONMENTS } from '$lib/stores/env-store';
	import { startUserSessionProxyDisabledReason } from '$lib/stores/reasons';
	import { taskStore } from '$lib/stores/task-store';
	import { userStore } from '$lib/stores/user-store';
	import { AwsEnv } from '$lib/types';
	import { invoke } from '@tauri-apps/api/core';

	let app = $state('');

	let env = $state(ENVIRONMENTS.at(0) ?? AwsEnv.DEV);
	function buildArn(address: string) {
		return `cookieSessionProxy::${address}::${env.toLowerCase()}`;
	}
	let address = $derived(
		`https://${app}${env == AwsEnv.PROD ? '' : '.' + env.toLowerCase()}.services.technipfmc.com`
	);
	let reason = $derived(startUserSessionProxyDisabledReason(address));
</script>

<form
	class="flex flex-row gap-2 mb-2"
	onsubmit={preventDefault(async () => {
		await taskStore.startTask({ name: address, arn: buildArn(address) }, async () => {
			console.log({
				address,
				env,
				headers: [
					{
						name: 'Origin',
						encodeBase64: false,
						value: address + '/'
					},
					{
						name: 'Referer',
						encodeBase64: false,
						value: address
					}
				]
			});
			return invoke('start_user_session_proxy', {
				address,
				env,
				headers: {
					['Origin']: address + '/',
					['Referer']: address
				}
			});
		});
		app = '';
	})}
>
	<input
		type="text"
		autocomplete="off"
		autocorrect="off"
		autocapitalize="off"
		spellcheck="false"
		placeholder="URL"
		bind:value={app}
		class="input input-bordered w-full max-w-xs input-sm"
	/>
	<select class="select-sm select max-w-xs select-bordered" bind:value={env}>
		{#each ENVIRONMENTS as env}
			<option value={env}>{env}</option>
		{/each}
	</select>
	<button
		class="btn btn-primary btn-sm"
		type="submit"
		disabled={$reason != null}
		data-umami-event="user_session_proxy_start"
		data-umami-event-uid={$userStore.id}
	>
		Start session proxy
	</button>
</form>
