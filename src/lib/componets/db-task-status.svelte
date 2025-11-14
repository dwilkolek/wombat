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
	let dialog: HTMLDialogElement | undefined = $state();
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
					data-umami-event={rdsAllowWrites ? 'dbeaver_dialog' : 'dbeaver_open'}
					data-umami-event-uid={$userStore.id}
					disabled={!$userStore.dbeaver_path}
					class={`link text-sm gap-1 text-amber-300 flex items-center ${
						$userStore.dbeaver_path ? 'hover:text-amber-500 cursor-pointer' : 'hover:text-red-900'
					}`}
					onclick={() => {
						if (rdsAllowWrites) {
							dialog?.show();
						} else {
							execute(
								'open_dbeaver',
								{
									db,
									port: port,
									readOnly: true
								},
								false
							);
						}
					}}
				>
					{port}
				</button>
			</div>
		</div>
	{:else}
		<span class="text-sm text-amber-300/60 animate-pulse">{port}</span>
	{/if}
{:else}
	<span class="text-sm text-gray-600">{port}</span>
{/if}

<dialog bind:this={dialog} onclose={() => console.log('closed')} class="modal">
	<div class="modal-box">
		<button
			class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2"
			onclick={() => dialog?.close()}>✕</button
		>
		<h3 class="text-lg font-bold">Open Dbeaver console to {db.identifier}</h3>
		<p class="py-4">
			All connections are opened with <span class="font-mono text-warning">autoCommit=false</span>.
			<br />
			Read only mode depends on DBeaver version and JDBC driver
			<span class="font-mono text-warning">readOnly</span> property.
		</p>
		<div class="modal-action">
			<button
				class="btn btn-info"
				data-umami-event="dbeaver_open_read"
				data-umami-event-uid={$userStore.id}
				disabled={!$userStore.dbeaver_path}
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

					dialog?.close();
				}}
			>
				Read only mode
			</button>
			<button
				class="btn btn-warning"
				data-umami-event="dbeaver_open_write"
				data-umami-event-uid={$userStore.id}
				disabled={!$userStore.dbeaver_path}
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
					dialog?.close();
				}}
			>
				Write mode
			</button>
		</div>
	</div>
</dialog>
