<script lang="ts">
	import JsonView from '$lib/componets/json-view.svelte';
	import { page } from '$app/stores';
	import { writeText } from '@tauri-apps/api/clipboard';

	$: log = JSON.parse(atob($page.url.searchParams.get('log')));
</script>

<svelte:head>
	<title>Logs Window</title>
	<meta name="description" content="Wombat" />
</svelte:head>
<div class="bg-base-300 min-h-screen">
	<button
		class="m-2 btn btn-active btn-primary btn-xs absolute right-2 top-2"
		on:click={async () => {
			await writeText(JSON.stringify(log, null, 2));
		}}>Copy raw json</button
	>
	<div class="p-2">
		<JsonView {log} nested={false} />
	</div>
</div>
