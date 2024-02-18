<script lang="ts">
	import { dbStore } from '$lib/stores/db-store';
	import type { ProxyEventMessage } from '$lib/stores/task-store';
	import DbTaskStatus from './db-task-status.svelte';
	export let task: ProxyEventMessage;
	$: db = dbStore.getDatabases(task.env).then((dbs) => dbs.find((db) => db.arn === task.arn));
</script>

{#await db then db}
	<div class="flex gap-2 items-center">
		<span class="italic text-sm">Database {task.env}:</span>

		{#if db}<DbTaskStatus task={task} db={db}/>{/if}
	</div>
{/await}
