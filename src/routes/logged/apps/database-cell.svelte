<script lang="ts">
	import { execute } from '$lib/error-store';
	import { taskStore } from '$lib/task-store';
	import type { DbInstance } from '$lib/types';
	import { userStore } from '$lib/user-store';
	import DbSecretBtn from '$lib/db-secret-btn.svelte';
	import { ask } from '@tauri-apps/api/dialog';
	import DbProxyStartBtn from '$lib/db-proxy-start-btn.svelte';
	import DbProxyStopBtn from '$lib/db-proxy-stop-btn.svelte';
	export let database: DbInstance | undefined;
	$: port = $taskStore.find((t) => t.arn == database?.arn)?.port
</script>

{#if database}
	<div class="flex flex-row items-center gap-1">
		{#if !port}
			<DbProxyStartBtn database={database} />
		{/if}
		{#if port}
			<DbProxyStopBtn database={database} port={port}/>
		{/if}
		<DbSecretBtn {database} />
	</div>
{/if}
{#if !database}
	<div />
{/if}
