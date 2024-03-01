<script lang="ts">
	import type { ProxyEventMessage } from '$lib/stores/task-store';
	import { userStore } from '$lib/stores/user-store';
	import type { EcsService } from '$lib/types';
	import { open } from '@tauri-apps/plugin-shell';

	export let task: ProxyEventMessage | undefined;
	export let service: EcsService;
	$: port = $userStore.service_proxy_port_map?.[service.name]?.[service.env] ?? '?';
</script>

{#if task}
	<div
		class={`tooltip tooltip-left flex items-center text-amber-300 hover:text-amber-500 gap-1`}
		data-tip={`Open ${task.name} in browser`}
	>
		{#if task.status !== 'STARTING'}
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
