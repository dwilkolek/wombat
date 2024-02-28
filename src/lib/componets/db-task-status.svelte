<script lang="ts">
	import { userStore } from '$lib/stores/user-store';
	import { execute } from '$lib/stores/error-store';
	import type { ProxyEventMessage } from '$lib/stores/task-store';
	import type { RdsInstance } from '$lib/types';
	export let task: ProxyEventMessage | undefined;
	export let db: RdsInstance;
	$: port = task?.port ?? $userStore.db_proxy_port_map?.[db.name]?.[db.env] ?? '?';
</script>

{#if task}
	{#if task.status !== 'STARTING'}
		<div
			class="tooltip tooltip-left"
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
					task &&
						execute(
							'open_dbeaver',
							{
								db,
								port: port
							},
							false
						);
				}}
			>
				{port}
			</button>
		</div>
	{:else}
		<span class="text-sm text-amber-300/[.6] animate-pulse">{port}</span>
	{/if}
{:else}
	<span class="text-sm text-gray-600">{port}</span>
{/if}
