<script lang="ts">
	import { TaskStatus, type Task } from '$lib/stores/task-store';
	import { userStore } from '$lib/stores/user-store';
	import type { AwsEnv } from '$lib/types';
	import { open } from '@tauri-apps/plugin-shell';

	interface Props {
		task: Task | undefined;
		app: string;
		env: AwsEnv;
	}

	let { task, app, env }: Props = $props();
	let port = $derived($userStore.lambda_app_proxy_port_map?.[app]?.[env] ?? '?');
</script>

{#if task}
	<div
		class="tooltip tooltip-left flex items-center text-amber-300 hover:text-amber-500 gap-1"
		data-tip={`Open ${task.name} in browser`}
	>
		{#if task.status === TaskStatus.RUNNING}
			<button
				data-umami-event="browser_lambda_app_proxy_open"
				data-umami-event-uid={$userStore.id}
				class="link text-sm"
				onclick={(e) => {
					e.preventDefault();
					open('http://localhost:' + task.port);
				}}
			>
				{task.port}</button
			>
		{:else}
			<span class="text-sm text-amber-300/[.6] animate-pulse">{port}</span>
		{/if}
	</div>
{:else}
	<span class="text-sm text-gray-600">{port}</span>
{/if}
