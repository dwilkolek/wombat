<script lang="ts">
	import { TaskStatus, type Task } from '$lib/stores/task-store';
	import { userStore } from '$lib/stores/user-store';
	import type { AwsEnv } from '$lib/types';
	import { open } from '@tauri-apps/api/shell';

	export let task: Task | undefined;
	export let app: string;
	export let env: AwsEnv;
	$: port = $userStore.lambda_app_proxy_port_map?.[app]?.[env] ?? '?';
</script>

{#if task}
	<div
		class={`tooltip tooltip-left flex items-center text-amber-300 hover:text-amber-500 gap-1`}
		data-tip={`Open ${task.name} in browser`}
	>
		{#if task.status === TaskStatus.RUNNING}
			<button
				class={`link text-sm`}
				on:click|preventDefault={() => {
					task && open('http://localhost:' + task.port);
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
