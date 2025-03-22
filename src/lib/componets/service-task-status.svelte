<script lang="ts">
	import { TaskStatus, type Task } from '$lib/stores/task-store';
	import { userStore } from '$lib/stores/user-store';
	import type { EcsService } from '$lib/types';
	import { open } from '@tauri-apps/plugin-shell';

	interface Props {
		task: Task | undefined;
		service: EcsService;
	}

	let { task, service }: Props = $props();
	let port = $derived($userStore.service_proxy_port_map?.[service.name]?.[service.env] ?? '?');
</script>

{#if task && task.status !== TaskStatus.FAILED}
	<div
		class="tooltip tooltip-left flex items-center text-amber-300 hover:text-amber-500 gap-1"
		data-tip={`Open ${task.name} in browser`}
	>
		{#if task.status !== TaskStatus.STARTING}
			<button
				class="link text-sm"
				onclick={(e) => {
					e.preventDefault();
					open('http://localhost:' + task.port);
				}}
				data-umami-event="browser_ecs_proxy_open"
				data-umami-event-uid={$userStore.id}
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
