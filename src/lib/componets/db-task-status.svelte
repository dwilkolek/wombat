<script lang="ts">
	import { userStore } from '$lib/stores/user-store';
	import { execute } from '$lib/stores/error-store';
	import { TaskStatus, type Task } from '$lib/stores/task-store';
	import { AwsEnv, type RdsInstance } from '$lib/types';
	import { featuresStore } from '$lib/stores/feature-store';
	interface Props {
		task: Task | undefined;
		db: RdsInstance;
	}

	let { task, db }: Props = $props();
	let port = $derived(task?.port ?? $userStore.db_proxy_port_map?.[db.name]?.[db.env] ?? '?');

	let rdsAllowWrites = $derived(
		db.env === AwsEnv.PROD
			? $featuresStore.rdsConnWrite && $featuresStore.rdsProdConnWrite
			: $featuresStore.rdsConnWrite
	);
</script>

{#if task && task.status !== TaskStatus.FAILED}
	{#if task.status !== TaskStatus.STARTING}
		<div class="flex gap-1">
			<div
				class="tooltip tooltip-left"
				data-tip={$userStore.dbeaver_path
					? 'Open connection in dbeaver'
					: 'Install dbeaver to get instant conneciton'}
			>
				<button
					data-umami-event="dbeaver_open"
					data-umami-event-uid={$userStore.id}
					disabled={!$userStore.dbeaver_path}
					class={`link text-sm gap-1 text-amber-300 flex items-center ${
						$userStore.dbeaver_path ? 'hover:text-amber-500 cursor-pointer' : 'hover:text-red-900'
					}`}
					onclick={() => {
						execute(
							'open_dbeaver',
							{
								db,
								port: port,
								readOnly: true
							},
							false
						);
					}}
				>
					{port}
				</button>
			</div>
			{#if rdsAllowWrites}
				<div
					class="tooltip tooltip-left"
					data-tip={$userStore.dbeaver_path
						? 'Open write connection in dbeaver'
						: 'Install dbeaver to get instant conneciton'}
				>
					<!-- svelte-ignore a11y_consider_explicit_label -->
					<button
						data-umami-event="dbeaver_open_write"
						data-umami-event-uid={$userStore.id}
						disabled={!$userStore.dbeaver_path}
						class={`link text-sm gap-1 text-amber-300 flex items-center ${
							$userStore.dbeaver_path ? 'hover:text-amber-500 cursor-pointer' : 'hover:text-red-900'
						}`}
						onclick={() => {
							execute(
								'open_dbeaver',
								{
									db,
									port: port,
									readOnly: false
								},
								false
							);
						}}
					>
						<svg
							xmlns="http://www.w3.org/2000/svg"
							fill="none"
							viewBox="0 0 24 24"
							stroke-width="1.5"
							stroke="currentColor"
							class="size-5"
						>
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								d="m16.862 4.487 1.687-1.688a1.875 1.875 0 1 1 2.652 2.652L10.582 16.07a4.5 4.5 0 0 1-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 0 1 1.13-1.897l8.932-8.931Zm0 0L19.5 7.125M18 14v4.75A2.25 2.25 0 0 1 15.75 21H5.25A2.25 2.25 0 0 1 3 18.75V8.25A2.25 2.25 0 0 1 5.25 6H10"
							/>
						</svg>
					</button>
				</div>
			{/if}
		</div>
	{:else}
		<span class="text-sm text-amber-300/[.6] animate-pulse">{port}</span>
	{/if}
{:else}
	<span class="text-sm text-gray-600">{port}</span>
{/if}
