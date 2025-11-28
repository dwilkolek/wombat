<script lang="ts">
	import { taskStore } from '$lib/stores/task-store';
	import type { RdsInstance } from '$lib/types';
	import DbSecretBtn from './db-secret-btn.svelte';
	import DbProxyStartBtn from './db-proxy-start-btn.svelte';
	import DbProxyStopBtn from './db-proxy-stop-btn.svelte';
	import DbSelfServiceChip from './db-self-service-chip.svelte';
	interface Props {
		database: RdsInstance;
	}

	let { database }: Props = $props();
	let port = $derived($taskStore.find((t) => t.arn == database?.arn)?.port);
</script>

{#if !port}
	<DbProxyStartBtn {database} />
{/if}
{#if port}
	<DbProxyStopBtn database_arn={database.arn} />
{/if}
<DbSecretBtn {database} />

<DbSelfServiceChip {database} />
