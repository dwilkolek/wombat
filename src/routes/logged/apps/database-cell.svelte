<script lang="ts">
	import { taskStore } from '$lib/task-store';
	import type { DbInstance } from '$lib/types';
	import DbSecretBtn from '$lib/db-secret-btn.svelte';
	import DbProxyStartBtn from '$lib/db-proxy-start-btn.svelte';
	import DbProxyStopBtn from '$lib/db-proxy-stop-btn.svelte';
	export let database: DbInstance;
	$: port = $taskStore.find((t) => t.arn == database?.arn)?.port;
</script>

<div class="flex flex-row items-center gap-1">
	{#if !port}
		<DbProxyStartBtn {database} />
	{/if}
	{#if port}
		<DbProxyStopBtn {database} {port} />
	{/if}
	<DbSecretBtn {database} />
</div>
