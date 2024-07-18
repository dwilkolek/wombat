<script lang="ts">
	import JsonView from '$lib/componets/json-view.svelte';
	import { page } from '$app/stores';
	import { writeText } from '@tauri-apps/api/clipboard';
	import { invoke } from '@tauri-apps/api/tauri';

	$: logPromise = invoke<string>('kv_get', { key: $page.url.searchParams.get('kvKey') });
</script>

<svelte:head>
	<title>Logs Window</title>
	<meta name="description" content="Wombat" />
</svelte:head>
<div class="bg-base-300 min-h-screen bg-opacity-100">
	{#await logPromise}
		Loading log: {$page.url.searchParams.get('kvKey')}
	{:then logString}
		{@const log = JSON.parse(logString)}
		<div class="p-2">
			<button
				class="m-2 btn btn-active btn-primary btn-xs absolute right-2 top-2"
				on:click={async () => {
					await writeText(JSON.stringify(log, null, 2));
				}}>Copy raw json</button
			>
			<JsonView {log} nested={false} />
		</div>
	{/await}
</div>
