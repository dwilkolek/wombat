<script lang="ts">
	import { userStore } from '$lib/stores/user-store';
	import { dbStore } from '$lib/stores/db-store';
	import { execute } from '$lib/stores/error-store';
	import type { ProxyEventMessage } from '$lib/stores/task-store';
	export let task: ProxyEventMessage;
	$: db = dbStore.getDatabases(task.env).then((dbs) => dbs.find((db) => db.arn === task.arn));
</script>

{#await db then db}
	<div class="flex gap-2 items-center">
		<span class="italic text-sm">Database {task.env}:</span>

		{#if task.status !== 'STARTING'}
			<div
				class="tooltip"
				data-tip={$userStore.dbeaver_path
					? 'Open connection in dbeaver'
					: 'Install dbeaver to get instant conneciton'}
			>
				<button
					disabled={!$userStore.dbeaver_path}
					class={`link text-sm gap-1 text-amber-300 flex items-center ${
						$userStore.dbeaver_path ? 'hover:text-amber-500 cursor-pointer' : 'hover:text-red-900'
					}`}
					on:click={() => {
						execute(
							'open_dbeaver',
							{
								db,
								port: task.port
							},
							false
						);
					}}
				>
					{task.port}
				</button>
			</div>
		{:else}
			<span class="text-sm text-amber-300">Starting...</span>
		{/if}
	</div>
{/await}
