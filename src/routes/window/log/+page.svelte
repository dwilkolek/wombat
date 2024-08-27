<script lang="ts">
	import JsonView from '$lib/componets/json-view.svelte';
	import { page } from '$app/stores';
	import { invoke } from '@tauri-apps/api/tauri';
	import { readable } from 'svelte/store';

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
			<JsonView log={readable(log)} nested={false} />
		</div>
	{/await}
</div>
