<script lang="ts">
	import AppEnvCard from '$lib/componets/app-env-card.svelte';
	import { writable } from 'svelte/store';
	import type { AppEnvPage } from './+page';
	import { app, invoke } from '@tauri-apps/api';
	import { listen } from '@tauri-apps/api/event';
	export let data: AppEnvPage;

	const logs = writable<{ message: string }[]>([]);

	let searchFilter: string = '';
	listen<string>('new-log-found', (newLog) => {
		let messageObj = JSON.parse(newLog.payload);
		console.log(messageObj);
		if (messageObj === newLog.payload) {
			logs.update((n) => [
				...n,
				{
					message: newLog.payload
				}
			]);
		} else {
			logs.update((n) => [
				...n,
				{
					message: messageObj.message
				}
			]);
		}
	});
</script>

<svelte:head>
	<title>APP {data.app}</title>
	<meta name="description" content="Wombat" />
</svelte:head>

<div>
	<AppEnvCard app={data.app} env={data.env} />
	<button
		on:click={() => {
			invoke('find_logs', { app: data.app, env: data.env });
		}}>RUN</button
	>
	<div>
		<input
			type="text"
			placeholder="Type here"
			class="input input-bordered w-full max-w-xs"
			bind:value={searchFilter}
		/>
		{searchFilter}
		{#each $logs as log}
			<div>{log.message}</div>
		{/each}
	</div>
</div>
